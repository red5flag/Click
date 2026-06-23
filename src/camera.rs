use crate::night_vision::NightVision;
use crate::recording::RecordingCommand;
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
    pub night_vision_enabled: bool,
    pub night_vision_threshold: u8,
    pub night_vision_history: usize,
}

impl CameraConfig {
    pub fn new(
        camera_index: i32,
        max_channel_depth: usize,
        night_vision_enabled: bool,
        night_vision_threshold: u8,
        night_vision_history: usize,
    ) -> Self {
        Self {
            camera_index,
            max_channel_depth,
            night_vision_enabled,
            night_vision_threshold,
            night_vision_history,
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
        recording_tx: mpsc::Sender<RecordingCommand>,
        shutdown: ShutdownSignal,
    ) -> Result<()> {
        info!("Initializing camera {}", self.config.camera_index);

        // Open camera in blocking task
        let camera_index = self.config.camera_index;
        let cap = tokio::task::spawn_blocking(move || {
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

        let mut night_vision = NightVision::new(
            self.config.night_vision_enabled,
            self.config.night_vision_threshold,
            self.config.night_vision_history,
        );

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
                    let empty = {
                        let fb = frame_buffer.lock().unwrap();
                        fb.empty()
                    };
                    if empty {
                        warn!("Empty frame received, retrying");
                        tokio::task::yield_now().await;
                        continue;
                    }

                    sequence += 1;
                    self.metrics.record_frame_captured();

                    let frame = {
                        let fb = frame_buffer.lock().unwrap();
                        Frame::new(fb.clone(), sequence)
                    };

                    // Automatic night vision based on frame brightness
                    {
                        let mut cap = cap.lock().unwrap();
                        night_vision.process_frame(frame.data.as_ref(), &mut *cap);
                    }

                    // Clone frame for recorder and send to inference pipeline.
                    // Use non-blocking sends so the camera keeps reading at full
                    // frame rate regardless of inference speed.
                    let recorder_frame = frame.clone_frame();
                    if let Err(e) = frame_tx.try_send(frame) {
                        debug!("Frame {} dropped from inference pipeline: {}", sequence, e);
                    } else {
                        debug!("Frame {} sent to inference pipeline", sequence);
                    }
                    if let Err(e) = recording_tx.try_send(RecordingCommand::WriteFrame(recorder_frame)) {
                        warn!("Frame {} dropped from recorder pipeline: {}", sequence, e);
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
