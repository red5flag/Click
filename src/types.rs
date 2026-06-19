use nalgebra::{Point2, Vector2};
use std::sync::Arc;

/// Detection box representing a detected person
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detection {
    /// Bounding box center point (normalized 0-1)
    pub center: Point2<f32>,
    /// Bounding box dimensions (normalized 0-1)
    pub dimensions: Vector2<f32>,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Class index (0 for person)
    pub class_id: usize,
}

impl Detection {
    /// Create a new detection
    pub fn new(cx: f32, cy: f32, w: f32, h: f32, confidence: f32, class_id: usize) -> Self {
        Self {
            center: Point2::new(cx, cy),
            dimensions: Vector2::new(w, h),
            confidence,
            class_id,
        }
    }

    /// Get top-left corner of bounding box
    pub fn top_left(&self) -> Point2<f32> {
        Point2::new(
            self.center.x - self.dimensions.x / 2.0,
            self.center.y - self.dimensions.y / 2.0,
        )
    }

    /// Get bottom-right corner of bounding box
    pub fn bottom_right(&self) -> Point2<f32> {
        Point2::new(
            self.center.x + self.dimensions.x / 2.0,
            self.center.y + self.dimensions.y / 2.0,
        )
    }

    /// Calculate intersection over union with another detection
    pub fn iou(&self, other: &Detection) -> f32 {
        let self_tl = self.top_left();
        let self_br = self.bottom_right();
        let other_tl = other.top_left();
        let other_br = other.bottom_right();

        // Calculate intersection
        let x1 = self_tl.x.max(other_tl.x);
        let y1 = self_tl.y.max(other_tl.y);
        let x2 = self_br.x.min(other_br.x);
        let y2 = self_br.y.min(other_br.y);

        let intersection_w = (x2 - x1).max(0.0);
        let intersection_h = (y2 - y1).max(0.0);
        let intersection_area = intersection_w * intersection_h;

        // Calculate union
        let self_area = self.dimensions.x * self.dimensions.y;
        let other_area = other.dimensions.x * other.dimensions.y;
        let union_area = self_area + other_area - intersection_area;

        if union_area <= 0.0 {
            0.0
        } else {
            intersection_area / union_area
        }
    }
}

/// A frame captured from the camera
#[derive(Debug)]
pub struct Frame {
    /// Frame data as Arc for cheap cloning
    pub data: Arc<opencv::core::Mat>,
    /// Timestamp when frame was captured
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Sequential frame number
    pub sequence: u64,
}

impl Frame {
    /// Create a new frame
    pub fn new(data: opencv::core::Mat, sequence: u64) -> Self {
        Self {
            data: Arc::new(data),
            timestamp: chrono::Utc::now(),
            sequence,
        }
    }

    /// Clone the frame (cheap due to Arc)
    pub fn clone_frame(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            timestamp: self.timestamp,
            sequence: self.sequence,
        }
    }
}

/// Inference result containing detections
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Detected persons
    pub detections: Vec<Detection>,
    /// Inference latency in milliseconds
    pub inference_latency_ms: f64,
    /// NMS latency in milliseconds
    pub nms_latency_ms: f64,
    /// Frame sequence number
    pub sequence: u64,
}

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    /// Idle, waiting for person detection
    Idle,
    /// Person detected, starting recording
    PersonDetected,
    /// Actively recording
    Recording,
    /// Grace period after person left
    RecordingGracePeriod,
}

/// Shutdown signal for graceful termination
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    inner: tokio::sync::watch::Sender<bool>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal
    pub fn new() -> (Self, tokio::sync::watch::Receiver<bool>) {
        let (tx, rx) = tokio::sync::watch::channel(false);
        (Self { inner: tx }, rx)
    }

    /// Create from existing sender
    pub fn from_sender(sender: tokio::sync::watch::Sender<bool>) -> Self {
        Self { inner: sender }
    }

    /// Trigger shutdown
    pub fn shutdown(&self) {
        let _ = self.inner.send(true);
    }

    /// Check if shutdown is requested
    pub fn is_shutdown(&self) -> bool {
        *self.inner.borrow()
    }

    /// Subscribe to shutdown signal
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<bool> {
        self.inner.subscribe()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        let (tx, _) = tokio::sync::watch::channel(false);
        Self { inner: tx }
    }
}

/// Performance metrics for a single frame processing
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameMetrics {
    /// Capture to inference queue wait time in ms
    pub queue_wait_ms: f64,
    /// Preprocessing time in ms
    pub preprocess_ms: f64,
    /// Inference time in ms
    pub inference_ms: f64,
    /// Postprocessing (NMS) time in ms
    pub postprocess_ms: f64,
    /// Total processing time in ms
    pub total_ms: f64,
}
