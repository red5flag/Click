use crate::types::{Frame, ShutdownSignal};
use anyhow::{Context, Result};
use opencv::prelude::*;
use opencv::videoio::{VideoCapture, VideoCaptureTrait};
use opencv::videoio::CAP_ANY;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Camera capture configuration
pub struct CameraConfig {
    pub camera_index: i32,
    pub max_channel_depth: usize,
}

impl CameraConfig {
    pub fn new(camera_index: i32, max_channel_depth: usize) -> Self {
        Self {
            camera_index,
            max_channel_depth,
        }
    }
}

/// Camera capture task
pub struct CameraCapture {
    config: CameraConfig,
    metrics: Arc<crate::metrics::MetricsCollector>,
}

impl CameraCapture {
    pub fn new(config: CameraConfig, metrics: Arc<crate::metrics::MetricsCollector>) -> Self {
        Self { config, metrics }
    }

    /// Start camera capture task
    pub async fn run(
        self,
        frame_tx: mpsc::Sender<Frame>,
        shutdown: ShutdownSignal,
    ) -> Result<()> {
        info!("Initializing camera {}", self.config.camera_index);

        // Open camera in blocking task
        let camera_index = self.config.camera_index;
        let mut cap = tokio::task::spawn_blocking(move || {
            VideoCapture::new(camera_index, CAP_ANY)
        })
        .await
        .context("Failed to spawn camera open task")?
        .context("Failed to open camera")?;

        // Get camera properties
        let width = cap.get(opencv::videoio::CAP_PROP_FRAME_WIDTH)? as i32;
        let height = cap.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT)? as i32;
        let fps = cap.get(opencv::videoio::CAP_PROP_FPS)?;

        info!(
            "Camera opened: {}x{} @ {:.1} FPS",
            width, height, fps
        );

        // Check if camera is opened
        if !cap.is_opened()? {
            return Err(anyhow::anyhow!("Camera failed to open"));
        }

        let cap = std::sync::Arc::new(std::sync::Mutex::new(cap));
        let mut sequence: u64 = 0;
        let frame_buffer = opencv::core::Mat::default();
        let frame_buffer = std::sync::Arc::new(std::sync::Mutex::new(frame_buffer));
        let shutdown_rx = shutdown.subscribe();

        info!("Camera capture started");

        loop {
            // Check for shutdown
            if *shutdown_rx.borrow() {
                info!("Camera capture shutting down");
                break;
            }

            // Read frame in blocking task
            let fb_clone = std::sync::Arc::clone(&frame_buffer);
            let cap_clone = std::sync::Arc::clone(&cap);
            let read_result = tokio::task::spawn_blocking(move || {
                let mut fb = fb_clone.lock().unwrap();
                let mut cap = cap_clone.lock().unwrap();
                cap.read(&mut *fb)
            })
            .await;

            match read_result {
                Ok(Ok(true)) => {
                    let fb = frame_buffer.lock().unwrap();
                    if fb.empty() {
                        warn!("Empty frame received, retrying");
                        tokio::task::yield_now().await;
                        continue;
                    }

                    sequence += 1;
                    self.metrics.record_frame_captured();

                    // Clone frame for sending
                    let frame = Frame::new(
                        fb.clone(),
                        sequence,
                    );

                    // Try to send with timeout - drop frame if channel full
                    match frame_tx.try_send(frame) {
                        Ok(()) => {
                            debug!("Frame {} sent to inference pipeline", sequence);
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            // Channel full - drop stale frame
                            warn!(
                                "Inference pipeline lagging, dropping frame {} (queue full)",
                                sequence
                            );
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            error!("Frame channel closed, stopping capture");
                            break;
                        }
                    }
                }
                Ok(Ok(false)) => {
                    warn!("Failed to read frame from camera, retrying");
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
                Ok(Err(e)) => {
                    error!("OpenCV error reading frame: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
                Err(e) => {
                    error!("Task panicked reading frame: {}", e);
                    break;
                }
            }
        }

        // Cleanup
        drop(cap);
        info!("Camera capture stopped ({} frames captured)", sequence);
        
        Ok(())
    }
}

/// Camera information for diagnostics
#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub index: i32,
    pub width: i32,
    pub height: i32,
    pub fps: f64,
}
