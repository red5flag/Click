use axum::{
    Router,
    routing::{get, post, delete},
};
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

mod handlers;
mod websocket;

use handlers::*;
use websocket::*;

#[derive(Clone, Default)]
pub struct WebAppState {
    pub camera_connected: bool,
    pub model_loaded: bool,
    pub recording: bool,
    pub persons_detected: usize,
    pub fps: f64,
    pub inference_time: f64,
    pub manual_recording: bool,
    pub auto_recording: bool,
    pub confidence_threshold: f64,
    pub iou_threshold: f64,
    pub grace_period_seconds: u64,
}

#[derive(Clone, Default, Serialize)]
pub struct FrameState {
    pub image: String,
    pub detections: Vec<crate::app::components::detection_overlay::DetectionBox>,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct WebState {
    pub app_state: Arc<RwLock<WebAppState>>,
    pub frame_tx: watch::Sender<FrameState>,
}

impl WebState {
    pub fn new() -> Self {
        Self {
            app_state: Arc::new(RwLock::new(WebAppState::default())),
            frame_tx: watch::channel(FrameState::default()).0,
        }
    }
}

pub fn create_router(state: WebState) -> Router {
    Router::new()
        .route("/ws/stream", get(video_websocket_handler))
        .route("/api/stream/snapshot", get(snapshot_handler))
        .route("/api/camera/settings", get(get_camera_settings).post(set_camera_settings))
        .route("/api/models", get(list_models))
        .route("/api/models/load", post(load_model_handler))
        .route("/api/detection/settings", get(get_detection_settings).post(set_detection_settings))
        .route("/api/tags", get(list_tags).post(create_tag))
        .route("/api/tags/{id}", delete(delete_tag_handler))
        .route("/api/recording/toggle", post(toggle_recording))
        .route("/api/auto-recording/toggle", post(toggle_auto_recording))
        .route("/api/recordings", get(list_recordings))
        .route("/api/stats", get(get_stats))
        .route("/api/settings", get(get_settings).post(save_settings))
        .route("/api/system/info", get(system_info))
        .route("/", get(index_handler))
        .fallback_service(ServeDir::new("target/site"))
        .with_state(state)
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}
