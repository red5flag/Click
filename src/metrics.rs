use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Thread-safe performance metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    frames_captured: Arc<AtomicU64>,
    frames_processed: Arc<AtomicU64>,
    inferences: Arc<AtomicU64>,
    total_inference_us: Arc<AtomicU64>,
    total_nms_us: Arc<AtomicU64>,
    start_time: Instant,
    last_report: Arc<std::sync::Mutex<Instant>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frames_captured: Arc::new(AtomicU64::new(0)),
            frames_processed: Arc::new(AtomicU64::new(0)),
            inferences: Arc::new(AtomicU64::new(0)),
            total_inference_us: Arc::new(AtomicU64::new(0)),
            total_nms_us: Arc::new(AtomicU64::new(0)),
            start_time: now,
            last_report: Arc::new(std::sync::Mutex::new(now)),
        }
    }

    pub fn record_frame_captured(&self) {
        self.frames_captured.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_frame_processed(&self) {
        self.frames_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_inference(&self, inference_us: u64, nms_us: u64) {
        self.inferences.fetch_add(1, Ordering::Relaxed);
        self.total_inference_us.fetch_add(inference_us, Ordering::Relaxed);
        self.total_nms_us.fetch_add(nms_us, Ordering::Relaxed);
    }

    pub fn report(&self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.start_time).as_secs_f64();
        let last = *self.last_report.lock().unwrap();
        let since_last = now.duration_since(last).as_secs_f64();

        if since_last < 1.0 {
            return;
        }

        let frames_captured = self.frames_captured.load(Ordering::Relaxed);
        let frames_processed = self.frames_processed.load(Ordering::Relaxed);
        let inferences = self.inferences.load(Ordering::Relaxed);
        let total_inference_us = self.total_inference_us.load(Ordering::Relaxed);
        let total_nms_us = self.total_nms_us.load(Ordering::Relaxed);

        let capture_fps = frames_captured as f64 / elapsed;
        let inference_fps = frames_processed as f64 / elapsed;

        let avg_inference_ms = if inferences > 0 {
            total_inference_us as f64 / inferences as f64 / 1000.0
        } else {
            0.0
        };

        let avg_nms_ms = if inferences > 0 {
            total_nms_us as f64 / inferences as f64 / 1000.0
        } else {
            0.0
        };

        info!(
            capture_fps = ?capture_fps,
            inference_fps = ?inference_fps,
            avg_inference_ms = ?avg_inference_ms,
            avg_nms_ms = ?avg_nms_ms,
            total_frames = frames_captured,
            total_inferences = inferences,
            "Performance metrics"
        );

        *self.last_report.lock().unwrap() = now;
    }

    pub fn get_summary(&self) -> MetricsSummary {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let frames_captured = self.frames_captured.load(Ordering::Relaxed);
        let frames_processed = self.frames_processed.load(Ordering::Relaxed);
        let inferences = self.inferences.load(Ordering::Relaxed);
        let total_inference_us = self.total_inference_us.load(Ordering::Relaxed);
        let total_nms_us = self.total_nms_us.load(Ordering::Relaxed);

        MetricsSummary {
            capture_fps: frames_captured as f64 / elapsed,
            inference_fps: frames_processed as f64 / elapsed,
            avg_inference_ms: if inferences > 0 {
                total_inference_us as f64 / inferences as f64 / 1000.0
            } else {
                0.0
            },
            avg_nms_ms: if inferences > 0 {
                total_nms_us as f64 / inferences as f64 / 1000.0
            } else {
                0.0
            },
            total_frames_captured: frames_captured,
            total_frames_processed: frames_processed,
            total_inferences: inferences,
            runtime_seconds: elapsed,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MetricsSummary {
    pub capture_fps: f64,
    pub inference_fps: f64,
    pub avg_inference_ms: f64,
    pub avg_nms_ms: f64,
    pub total_frames_captured: u64,
    pub total_frames_processed: u64,
    pub total_inferences: u64,
    pub runtime_seconds: f64,
}

impl std::fmt::Display for MetricsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Capture: {:.1} FPS | Inference: {:.1} FPS | \
             Avg Inference: {:.2}ms | Avg NMS: {:.2}ms | \
             Runtime: {:.1}s",
            self.capture_fps,
            self.inference_fps,
            self.avg_inference_ms,
            self.avg_nms_ms,
            self.runtime_seconds
        )
    }
}
