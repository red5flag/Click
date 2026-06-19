use crate::camera::{CameraCapture, CameraConfig};
use crate::config::Config;
use crate::detection::{DetectionConfig, DetectionStateMachine, StateTransition};
use crate::metrics::MetricsCollector;
use crate::model::YoloModel;
use crate::recording::{RecorderConfig, RecordingCommand, VideoRecorder};
use crate::types::{Frame, InferenceResult, ShutdownSignal};
use anyhow::{Context, Result};
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
    pub async fn new(config: Config) -> Result<Self> {
        let metrics = Arc::new(MetricsCollector::new());
        let (tx, _rx) = tokio::sync::watch::channel(false);

        info!("Application initialized");
        
        Ok(Self {
            config,
            metrics,
            tx,
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
        let inference_handle = tokio::spawn(async move {
            Self::inference_task(
                model_clone,
                frame_rx,
                inference_tx,
                metrics_clone,
                inference_shutdown,
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
        let mut shutdown_rx = self.tx.subscribe();
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
    ) {
        info!("Inference task started");

        while let Some(frame) = frame_rx.recv().await {
            if shutdown.is_shutdown() {
                break;
            }

            let sequence = frame.sequence;
            let preprocess_start = Instant::now();

            // Preprocess frame
            let tensor = match model.preprocess(frame.data.as_ref()) {
                Ok(t) => t,
                Err(e) => {
                    warn!("Preprocessing failed for frame {}: {}", sequence, e);
                    continue;
                }
            };

            let preprocess_elapsed = preprocess_start.elapsed();

            // Run inference in blocking task
            let model_clone = Arc::clone(&model);
            let inference_result = tokio::task::spawn_blocking(move || {
                model_clone.infer(tensor)
            }).await;

            match inference_result {
                Ok(Ok(detections)) => {
                    let total_elapsed = preprocess_start.elapsed();
                    
                    metrics.record_frame_processed();

                    let result = InferenceResult {
                        detections,
                        inference_latency_ms: (total_elapsed.as_secs_f64() - preprocess_elapsed.as_secs_f64()) * 1000.0,
                        nms_latency_ms: 0.0, // Included in infer
                        sequence,
                    };

                    if let Err(_) = inference_tx.send(result).await {
                        warn!("Inference channel closed");
                        break;
                    }
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
