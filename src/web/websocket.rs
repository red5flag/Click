use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    extract::State,
    response::Response,
};
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
    let mut frame_rx = state.frame_tx.subscribe();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let frame = frame_rx.borrow().clone();
                if let Ok(data) = serde_json::to_string(&frame) {
                    if socket.send(Message::Text(data.into())).await.is_err() {
                        break;
                    }
                }
            }
            _ = frame_rx.changed() => {
                // Frame state updated; next tick will send the latest value
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

pub async fn snapshot_handler(State(_state): State<WebState>) -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
        String::new(),
    )
}
