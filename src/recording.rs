use crate::types::{Frame, ShutdownSignal};
use anyhow::{Context, Result};
use chrono::Local;
use opencv::prelude::*;
use opencv::core::Size;
use opencv::videoio::VideoWriter;
use opencv::videoio::VideoWriterTrait;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Video recorder configuration
#[derive(Debug, Clone)]
pub struct RecorderConfig {
    pub recordings_directory: String,
    pub video_codec: String,
    pub recording_fps: f64,
}

impl RecorderConfig {
    pub fn new(recordings_directory: String, video_codec: String, recording_fps: f64) -> Self {
        Self {
            recordings_directory,
            video_codec,
            recording_fps,
        }
    }
}

/// Video recording task
pub struct VideoRecorder {
    config: RecorderConfig,
    current_writer: Option<VideoWriter>,
    current_path: Option<PathBuf>,
    frame_count: u64,
}

/// Recording control commands
#[derive(Debug)]
pub enum RecordingCommand {
    /// Start recording with filename base
    Start { timestamp: chrono::DateTime<chrono::Utc> },
    /// Write a frame
    WriteFrame(Frame),
    /// Stop and finalize recording
    Stop,
}

impl VideoRecorder {
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            config,
            current_writer: None,
            current_path: None,
            frame_count: 0,
        }
    }

    /// Ensure recordings directory exists
    fn ensure_directory(&self) -> Result<()> {
        let path = Path::new(&self.config.recordings_directory);
        if !path.exists() {
            info!(
                "Creating recordings directory: {}",
                path.display()
            );
            fs::create_dir_all(path).with_context(|| {
                format!("Failed to create recordings directory: {}", path.display())
            })?;
        }
        Ok(())
    }

    /// Generate filename from timestamp
    fn generate_filename(&self, timestamp: chrono::DateTime<chrono::Utc>) -> PathBuf {
        let filename = format!("{}.mp4", timestamp.format("%Y-%m-%d_%H-%M-%S"));
        Path::new(&self.config.recordings_directory).join(filename)
    }

    /// Get OpenCV fourcc code for codec
    fn get_fourcc(&self) -> i32 {
        match self.config.video_codec.as_str() {
            "h264" | "avc1" => {
                // Try H.264
                opencv::videoio::VideoWriter::fourcc('a', 'v', 'c', '1')
                    .or_else(|_| opencv::videoio::VideoWriter::fourcc('h', '2', '6', '4'))
                    .unwrap_or(0)
            }
            "h265" | "hevc" => {
                opencv::videoio::VideoWriter::fourcc('h', 'e', 'v', 'c')
                    .or_else(|_| opencv::videoio::VideoWriter::fourcc('h', '2', '6', '5'))
                    .unwrap_or(0)
            }
            "mjpeg" | "mjpg" => {
                opencv::videoio::VideoWriter::fourcc('m', 'j', 'p', 'g').unwrap_or(0)
            }
            "xvid" => opencv::videoio::VideoWriter::fourcc('x', 'v', 'i', 'd').unwrap_or(0),
            _ => {
                warn!("Unknown codec '{}', trying H.264", self.config.video_codec);
                opencv::videoio::VideoWriter::fourcc('a', 'v', 'c', '1')
                    .unwrap_or(0)
            }
        }
    }

    /// Start new recording
    fn start_recording(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>,
        frame: &Frame,
    ) -> Result<()> {
        self.ensure_directory()?;

        let path = self.generate_filename(timestamp);
        info!("Starting recording: {}", path.display());

        let fourcc = self.get_fourcc();
        if fourcc == 0 {
            return Err(anyhow::anyhow!("Failed to get valid fourcc codec"));
        }

        let mat = frame.data.as_ref();
        let width = mat.cols();
        let height = mat.rows();

        if width == 0 || height == 0 {
            return Err(anyhow::anyhow!("Invalid frame dimensions: {}x{}", width, height));
        }

        debug!(
            "Video parameters: {}x{} @ {:.1} FPS, codec: {}",
            width, height, self.config.recording_fps, self.config.video_codec
        );

        let mut writer = VideoWriter::new(
            path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path"))?,
            fourcc,
            self.config.recording_fps,
            Size::new(width, height),
            true, // isColor
        )?;

        if !writer.is_opened()? {
            // Try fallback to MJPEG
            warn!("Failed to open H.264 writer, trying MJPEG fallback");
            let mjpeg_fourcc = opencv::videoio::VideoWriter::fourcc('m', 'j', 'p', 'g')?;
            writer = VideoWriter::new(
                path.to_str().unwrap(),
                mjpeg_fourcc,
                self.config.recording_fps,
                Size::new(width, height),
                true,
            )?;

            if !writer.is_opened()? {
                return Err(anyhow::anyhow!("Failed to open any video codec"));
            }
        }

        self.current_writer = Some(writer);
        self.current_path = Some(path);
        self.frame_count = 0;

        // Write first frame
        self.write_frame_internal(frame)?;

        info!("Recording started successfully");
        Ok(())
    }

    /// Write frame to current recording
    fn write_frame_internal(&mut self, frame: &Frame) -> Result<()> {
        if let Some(ref mut writer) = self.current_writer {
            writer.write(frame.data.as_ref())?;
            self.frame_count += 1;
            debug!("Wrote frame {} to recording", self.frame_count);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active recording"))
        }
    }

    /// Stop and finalize recording
    fn stop_recording(&mut self) -> Result<()> {
        if let Some(mut writer) = self.current_writer.take() {
            let path = self.current_path.take();
            let count = self.frame_count;

            // Release writer to flush file
            drop(writer);

            if let Some(ref p) = path {
                info!(
                    "Recording finalized: {} ({} frames)",
                    p.display(),
                    count
                );
            }
        }
        Ok(())
    }

    /// Run recorder task
    pub async fn run(
        mut self,
        mut cmd_rx: mpsc::Receiver<RecordingCommand>,
        _shutdown: ShutdownSignal,
    ) -> Result<()> {
        info!("Video recorder task started");

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                RecordingCommand::Start { timestamp } => {
                    // Note: We'll get the first frame with WriteFrame
                    // Just store the timestamp for when recording actually starts
                    debug!("Recording start requested at {}", timestamp);
                }
                RecordingCommand::WriteFrame(frame) => {
                    // Auto-start recording if needed
                    if self.current_writer.is_none() {
                        if let Err(e) = self.start_recording(frame.timestamp, &frame) {
                            error!("Failed to start recording: {}", e);
                            continue;
                        }
                    }

                    if let Err(e) = self.write_frame_internal(&frame) {
                        error!("Failed to write frame: {}", e);
                    }
                }
                RecordingCommand::Stop => {
                    if let Err(e) = self.stop_recording() {
                        error!("Failed to stop recording: {}", e);
                    }
                }
            }
        }

        // Final cleanup
        let _ = self.stop_recording();
        info!("Video recorder task stopped");

        Ok(())
    }
}

/// Current recording state information
#[derive(Debug, Clone)]
pub struct RecordingInfo {
    pub active: bool,
    pub path: Option<PathBuf>,
    pub frame_count: u64,
    pub duration_seconds: f64,
}
