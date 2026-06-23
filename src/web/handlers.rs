use axum::{
    extract::{State, Path},
    response::Html,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use super::{WebState, ApiResponse};

pub async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../../index.html"))
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct CameraSettings {
    pub camera_index: i32,
    pub resolution: String,
    pub fps: i32,
    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
    pub auto_exposure: bool,
    pub auto_white_balance: bool,
    pub auto_gain: bool,
}

pub async fn get_camera_settings(State(_state): State<WebState>) -> Json<ApiResponse<CameraSettings>> {
    Json(ApiResponse::success(CameraSettings::default()))
}

pub async fn set_camera_settings(
    State(_state): State<WebState>,
    Json(settings): Json<CameraSettings>,
) -> Json<ApiResponse<()>> {
    println!("Camera settings: {:?}", settings);
    Json(ApiResponse::success(()))
}

#[derive(Serialize, Deserialize, Default)]
pub struct ModelInfo {
    pub path: String,
    pub name: String,
    pub size_mb: f64,
    pub model_type: String,
    pub input_shape: String,
    pub num_classes: i32,
    pub version: String,
}

pub async fn list_models(State(_state): State<WebState>) -> Json<ApiResponse<Vec<ModelInfo>>> {
    let mut models = Vec::new();
    if let Ok(entries) = std::fs::read_dir("./models") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("onnx") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let size_mb = entry
                    .metadata()
                    .map(|m| m.len() as f64 / (1024.0 * 1024.0))
                    .unwrap_or(0.0);
                models.push(ModelInfo {
                    path: path.display().to_string(),
                    name: name.clone(),
                    size_mb,
                    model_type: "ONNX".to_string(),
                    input_shape: "640x640".to_string(),
                    num_classes: 80,
                    version: name,
                });
            }
        }
    }
    if models.is_empty() {
        models.push(ModelInfo {
            path: "./models/model.onnx".to_string(),
            name: "YOLO".to_string(),
            size_mb: 6.2,
            model_type: "ONNX".to_string(),
            input_shape: "640x640".to_string(),
            num_classes: 80,
            version: "yolo".to_string(),
        });
    }
    Json(ApiResponse::success(models))
}

pub async fn load_model_handler(
    State(_state): State<WebState>,
    Json(params): Json<HashMap<String, String>>,
) -> Json<ApiResponse<()>> {
    println!("Load model: {:?}", params);
    Json(ApiResponse::success(()))
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DetectionSettings {
    pub confidence_threshold: f64,
    pub iou_threshold: f64,
    pub grace_period_seconds: i32,
}

pub async fn get_detection_settings(State(state): State<WebState>) -> Json<ApiResponse<DetectionSettings>> {
    let app_state = state.app_state.read().await;
    Json(ApiResponse::success(DetectionSettings {
        confidence_threshold: app_state.confidence_threshold,
        iou_threshold: app_state.iou_threshold,
        grace_period_seconds: app_state.grace_period_seconds as i32,
    }))
}

pub async fn set_detection_settings(
    State(state): State<WebState>,
    Json(settings): Json<DetectionSettings>,
) -> Json<ApiResponse<DetectionSettings>> {
    let mut app_state = state.app_state.write().await;
    app_state.confidence_threshold = settings.confidence_threshold.clamp(0.0, 1.0);
    app_state.iou_threshold = settings.iou_threshold.clamp(0.0, 1.0);
    app_state.grace_period_seconds = settings.grace_period_seconds.max(0) as u64;
    info!(
        "Detection settings updated: confidence={:.2}, iou={:.2}, grace={}s",
        app_state.confidence_threshold,
        app_state.iou_threshold,
        app_state.grace_period_seconds
    );
    Json(ApiResponse::success(settings))
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub usage_count: i32,
    pub category: String,
}

pub async fn list_tags(State(_state): State<WebState>) -> Json<ApiResponse<Vec<Tag>>> {
    Json(ApiResponse::success(vec![
        Tag { id: "1".to_string(), name: "Person".to_string(), color: "#22c55e".to_string(), created_at: "2024-01-15".to_string(), usage_count: 42, category: "General".to_string() },
    ]))
}

pub async fn create_tag(State(_state): State<WebState>, Json(tag): Json<Tag>) -> Json<ApiResponse<Tag>> {
    Json(ApiResponse::success(tag))
}

pub async fn delete_tag_handler(State(_state): State<WebState>, Path(_id): Path<String>) -> Json<ApiResponse<()>> {
    Json(ApiResponse::success(()))
}

pub async fn toggle_recording(State(state): State<WebState>) -> Json<ApiResponse<bool>> {
    let new_state = {
        let mut app_state = state.app_state.write().await;
        app_state.manual_recording = !app_state.manual_recording;
        app_state.manual_recording
    };
    info!("Manual recording toggled: {}", new_state);
    Json(ApiResponse::success(new_state))
}

#[derive(Serialize, Default)]
pub struct RecordingInfo {
    pub id: String,
    pub filename: String,
}

pub async fn list_recordings(State(_state): State<WebState>) -> Json<ApiResponse<Vec<RecordingInfo>>> {
    Json(ApiResponse::success(vec![]))
}

#[derive(Serialize)]
pub struct SystemStats {
    pub persons_detected: usize,
    pub fps: f64,
    pub inference_time: f64,
    pub recording: bool,
    pub manual_recording: bool,
    pub auto_recording: bool,
    pub storage_used: f64,
}

pub async fn get_stats(State(state): State<WebState>) -> Json<ApiResponse<SystemStats>> {
    let app_state = state.app_state.read().await;
    Json(ApiResponse::success(SystemStats {
        persons_detected: app_state.persons_detected,
        fps: app_state.fps,
        inference_time: app_state.inference_time,
        recording: app_state.recording,
        manual_recording: app_state.manual_recording,
        auto_recording: app_state.auto_recording,
        storage_used: 45.2,
    }))
}

pub async fn toggle_auto_recording(State(state): State<WebState>) -> Json<ApiResponse<bool>> {
    let new_state = {
        let mut app_state = state.app_state.write().await;
        app_state.auto_recording = !app_state.auto_recording;
        app_state.auto_recording
    };
    info!("Auto-recording toggled: {}", new_state);
    Json(ApiResponse::success(new_state))
}

pub async fn get_settings(State(_state): State<WebState>) -> Json<ApiResponse<HashMap<String, serde_json::Value>>> {
    Json(ApiResponse::success(HashMap::new()))
}

pub async fn save_settings(
    State(_state): State<WebState>,
    Json(settings): Json<HashMap<String, serde_json::Value>>,
) -> Json<ApiResponse<()>> {
    println!("Settings: {:?}", settings);
    Json(ApiResponse::success(()))
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub platform: String,
}

pub async fn system_info(State(_state): State<WebState>) -> Json<ApiResponse<SystemInfo>> {
    Json(ApiResponse::success(SystemInfo {
        version: "1.0.0".to_string(),
        platform: "Linux".to_string(),
    }))
}
