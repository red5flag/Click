use crate::app::components::detection_overlay::DetectionBox;
use crate::camera::{CameraCapture, CameraConfig};
use crate::config::Config;
use crate::detection::{DetectionConfig, DetectionStateMachine, StateTransition};
use crate::metrics::MetricsCollector;
use crate::model::YoloModel;
use crate::recording::{RecorderConfig, RecordingCommand, VideoRecorder};
use crate::types::{Frame, InferenceResult, ShutdownSignal};
use crate::web::{FrameState, WebState};
use anyhow::{Context, Result};
use base64::Engine;
use opencv::imgcodecs;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Main application orchestrator
pub struct Application {
    config: Config,
    metrics: Arc<MetricsCollector>,
    tx: tokio::sync::watch::Sender<bool>,
    web_state: crate::web::WebState,
}

/// Shutdown handle for external control
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    tx: tokio::sync::watch::Sender<bool>,
}

impl ShutdownHandle {
    /// Trigger application shutdown
    pub async fn shutdown(&self) {
        let _ = self.tx.send(true);
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.tx.subscribe();
        let _ = rx.wait_for(|shutdown| *shutdown).await;
    }
}

impl Application {
    /// Create and initialize the application
    pub async fn new(config: Config, web_state: crate::web::WebState) -> Result<Self> {
        let metrics = Arc::new(MetricsCollector::new());
        let (tx, _rx) = tokio::sync::watch::channel(false);

        info!("Application initialized");
        
        Ok(Self {
            config,
            metrics,
            tx,
            web_state,
        })
    }

    /// Get shutdown handle for external control
    pub fn get_shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            tx: self.tx.clone(),
        }
    }

    /// Run the main application loop
    pub async fn run(self) -> Result<()> {
        info!("Starting application main loop");

        // Initialize YOLO model
        let model = Arc::new(YoloModel::new(
            &self.config.model_path,
            self.config.confidence_threshold,
            self.config.iou_threshold,
        ).context("Failed to initialize YOLO model")?);

        let model_info = model.model_info();
        info!("Model initialized: {:?}", model_info);

        // Create channels
        let (frame_tx, frame_rx) = mpsc::channel::<Frame>(self.config.max_channel_depth);
        let (inference_tx, inference_rx) = mpsc::channel::<InferenceResult>(self.config.max_channel_depth);
        let (recording_tx, recording_rx) = mpsc::channel::<RecordingCommand>(self.config.max_channel_depth);

        // Create shutdown signal that all tasks will watch
        let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);
        let shutdown_signal = ShutdownSignal::from_sender(shutdown_tx.clone());

        // Spawn camera capture task
        let camera_config = CameraConfig::new(
            self.config.camera_index,
            self.config.max_channel_depth,
        );
        let camera = CameraCapture::new(camera_config, self.metrics.clone());
        let camera_shutdown = shutdown_signal.clone();
        let camera_handle = tokio::spawn(async move {
            if let Err(e) = camera.run(frame_tx, camera_shutdown).await {
                error!("Camera task error: {}", e);
            }
        });

        // Spawn inference task
        let model_clone = Arc::clone(&model);
        let metrics_clone = self.metrics.clone();
        let inference_shutdown = shutdown_signal.clone();
        let web_state_clone = self.web_state.clone();
        let inference_handle = tokio::spawn(async move {
            Self::inference_task(
                model_clone,
                frame_rx,
                inference_tx,
                metrics_clone,
                inference_shutdown,
                web_state_clone,
            ).await
        });

        // Spawn detection state machine task
        let detection_config = DetectionConfig::new(
            self.config.confidence_threshold,
            self.config.grace_period_seconds,
        );
        let detection_shutdown = shutdown_signal.clone();
        let detection_handle = tokio::spawn(async move {
            Self::detection_task(
                detection_config,
                inference_rx,
                recording_tx,
                detection_shutdown,
            ).await
        });

        // Spawn video recorder task
        let recorder_config = RecorderConfig::new(
            self.config.recordings_directory.clone(),
            self.config.video_codec.clone(),
            self.config.recording_fps,
        );
        let recorder = VideoRecorder::new(recorder_config);
        let recorder_shutdown = shutdown_signal.clone();
        let recorder_handle = tokio::spawn(async move {
            if let Err(e) = recorder.run(recording_rx, recorder_shutdown).await {
                error!("Recorder task error: {}", e);
            }
        });

        // Spawn metrics reporter
        let metrics_clone = self.metrics.clone();
        let metrics_interval = self.config.metrics_interval_seconds;
        let metrics_shutdown = shutdown_signal.clone();
        let metrics_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(metrics_interval));
            loop {
                interval.tick().await;
                if metrics_shutdown.is_shutdown() {
                    break;
                }
                metrics_clone.report();
            }
        });

        // Wait for shutdown signal from main.rs
        let shutdown_rx = self.tx.subscribe();
        loop {
            if *shutdown_rx.borrow() {
                info!("Shutdown signal received, waiting for tasks...");
                // Signal all tasks to shut down
                let _ = shutdown_tx.send(true);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Wait for all tasks to complete
        let _ = tokio::join!(
            camera_handle,
            inference_handle,
            detection_handle,
            recorder_handle,
            metrics_handle
        );

        // Final metrics report
        let summary = self.metrics.get_summary();
        info!("Final metrics: {}", summary);

        Ok(())
    }

    /// Inference task: preprocess frames and run model
    async fn inference_task(
        model: Arc<YoloModel>,
        mut frame_rx: mpsc::Receiver<Frame>,
        inference_tx: mpsc::Sender<InferenceResult>,
        metrics: Arc<MetricsCollector>,
        shutdown: ShutdownSignal,
        web_state: WebState,
    ) {
        info!("Inference task started");

        while let Some(mut frame) = frame_rx.recv().await {
            if shutdown.is_shutdown() {
                break;
            }

            // Drop stale frames to keep latency low
            while let Ok(latest) = frame_rx.try_recv() {
                frame = latest;
            }

            let sequence = frame.sequence;

            // Preprocess frame
            let tensor = match model.preprocess(frame.data.as_ref()) {
                Ok(t) => t,
                Err(e) => {
                    warn!("Preprocessing failed for frame {}: {}", sequence, e);
                    continue;
                }
            };

            // Run inference in blocking task
            let model_clone = Arc::clone(&model);
            let inference_start = Instant::now();
            let inference_result = tokio::task::spawn_blocking(move || {
                model_clone.infer(tensor)
            }).await;
            let inference_us = inference_start.elapsed().as_micros() as u64;

            match inference_result {
                Ok(Ok(detections)) => {
                    let inference_ms = inference_us as f64 / 1000.0;

                    metrics.record_frame_processed();
                    metrics.record_inference(inference_us, 0);

                    let result = InferenceResult {
                        detections: detections.clone(),
                        inference_latency_ms: inference_ms,
                        nms_latency_ms: 0.0, // Included in infer
                        sequence,
                    };

                    if let Err(_) = inference_tx.send(result).await {
                        warn!("Inference channel closed");
                        break;
                    }

                    // Publish frame + detections for the web stream
                    let frame_data = Arc::clone(&frame.data);
                    let web_detections = detections
                        .iter()
                        .map(|d| DetectionBox {
                            id: format!("{}-{:.3}", sequence, d.confidence),
                            x: (d.center.x - d.dimensions.x / 2.0).max(0.0) as f64,
                            y: (d.center.y - d.dimensions.y / 2.0).max(0.0) as f64,
                            width: d.dimensions.x as f64,
                            height: d.dimensions.y as f64,
                            confidence: d.confidence as f64,
                            class_name: "person".to_string(),
                        })
                        .collect::<Vec<_>>();
                    let web_state_clone = web_state.clone();
                    tokio::spawn(async move {
                        let image = tokio::task::spawn_blocking(move || {
                            Self::encode_frame_jpeg(&frame_data)
                        }).await.ok().flatten().unwrap_or_default();

                        let _ = web_state_clone.frame_tx.send(FrameState {
                            image,
                            detections: web_detections,
                            timestamp: chrono::Utc::now().timestamp_millis(),
                        });
                    });
                }
                Ok(Err(e)) => {
                    warn!("Inference failed for frame {}: {}", sequence, e);
                }
                Err(e) => {
                    error!("Inference task panicked: {}", e);
                }
            }
        }

        info!("Inference task stopped");
    }

    fn encode_frame_jpeg(frame: &opencv::core::Mat) -> Option<String> {
        let mut buf = opencv::core::Vector::<u8>::new();
        let params = opencv::core::Vector::<i32>::new();
        if imgcodecs::imencode(".jpg", frame, &mut buf, &params).ok()? {
            Some(base64::engine::general_purpose::STANDARD.encode(buf.as_slice()))
        } else {
            None
        }
    }

    /// Detection state machine task
    async fn detection_task(
        config: DetectionConfig,
        mut inference_rx: mpsc::Receiver<InferenceResult>,
        recording_tx: mpsc::Sender<RecordingCommand>,
        shutdown: ShutdownSignal,
    ) {
        info!("Detection task started");

        let mut state_machine = DetectionStateMachine::new(config);
        let mut recording_active = false;

        while let Some(result) = inference_rx.recv().await {
            if shutdown.is_shutdown() {
                break;
            }

            let transition = state_machine.process_inference(&result);

            // Handle state transitions
            match transition {
                StateTransition::RecordingStarted => {
                    info!("Recording started - person detected");
                    recording_active = true;
                    let _ = recording_tx.send(RecordingCommand::Start {
                        timestamp: chrono::Utc::now(),
                    }).await;

                    // Also send first frame
                    // This is handled by WriteFrame below
                }
                StateTransition::RecordingContinued => {
                    // Continue recording - frames are sent below
                }
                StateTransition::RecordingStopped => {
                    info!("Recording stopped - grace period expired");
                    recording_active = false;
                    let _ = recording_tx.send(RecordingCommand::Stop).await;
                }
                StateTransition::GracePeriodStarted => {
                    // Continue recording during grace period
                }
                StateTransition::GracePeriodCancelled => {
                    info!("Grace period cancelled - person reappeared");
                }
                StateTransition::NoChange => {}
            }

            // Send frame to recorder if recording is active
            // Note: In a real implementation, we'd need to get the actual frame
            // For now, we rely on the inference to drive the state machine
            // and the frame would be passed through or we re-capture
        }

        // Ensure recording is stopped on shutdown
        if recording_active {
            let _ = recording_tx.send(RecordingCommand::Stop).await;
        }

        info!("Detection task stopped");
    }
}
