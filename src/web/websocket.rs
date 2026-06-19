use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    extract::State,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;

use super::WebState;

pub async fn video_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_video_socket(socket, state))
}

async fn handle_video_socket(mut socket: WebSocket, state: WebState) {
    let _ = socket.send(Message::Text(json!({
        "type": "connected",
        "message": "Video stream established"
    }).to_string().into())).await;
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let frame_data = get_frame_data(&state).await;
                if let Ok(data) = serde_json::to_string(&frame_data) {
                    if socket.send(Message::Text(data.into())).await.is_err() {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

async fn get_frame_data(state: &WebState) -> serde_json::Value {
    let app_state = state.app_state.read().await;
    json!({
        "type": "frame",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "persons_detected": app_state.persons_detected,
        "fps": app_state.fps,
        "recording": app_state.recording,
        "image": "",
        "detections": []
    })
}

pub async fn snapshot_handler(State(_state): State<WebState>) -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
        String::new(),
    )
}
