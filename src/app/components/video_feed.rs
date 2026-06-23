use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::Show;

#[cfg(target_arch = "wasm32")]
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, WebSocket};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use super::detection_overlay::{DetectionBox, DetectionOverlay};

#[component]
pub fn VideoFeed() -> impl IntoView {
    let (stream_active, set_stream_active) = signal(false);
    let (frame_data, set_frame_data) = signal::<Option<String>>(None);
    let (detections, set_detections) = signal::<Vec<DetectionBox>>(vec![]);
    let (recording, set_recording) = signal(false);
    let (auto_recording, set_auto_recording) = signal(true);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    #[cfg(target_arch = "wasm32")]
    {
        manage_websocket(stream_active, set_frame_data, set_detections);
        poll_recording_state(set_recording, set_auto_recording);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = set_frame_data;
        let _ = set_detections;
        let _ = set_recording;
        let _ = set_auto_recording;
    }
    
    Effect::new(move |_| {
        if let Some(data) = frame_data.get() {
            if let Some(canvas) = canvas_ref.get() {
                #[cfg(target_arch = "wasm32")]
                {
                    let canvas_el = canvas.unchecked_ref::<HtmlCanvasElement>();
                    render_frame_to_canvas(&canvas_el, &data, detections.get());
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = &canvas;
                    let _ = &data;
                    let _ = detections.get();
                }
            }
        }
    });

    let toggle_recording = move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/recording/toggle").send().await;
        });
    };

    let toggle_auto_recording = move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/auto-recording/toggle").send().await;
        });
    };
    
    view! {
        <div class="video-feed-container">
            <div class="video-header">
                <h3>"Live Feed"</h3>
                <div class="video-controls">
                    <button
                        class={move || if stream_active.get() { "btn-stop" } else { "btn-start" }}
                        on:click=move |_| set_stream_active.update(|v| *v = !*v)
                    >
                        {move || if stream_active.get() { "Stop Stream" } else { "Start Stream" }}
                    </button>
                    <button
                        class={move || if recording.get() { "btn-stop" } else { "btn-primary" }}
                        on:click=toggle_recording
                    >
                        {move || if recording.get() { "Stop Recording" } else { "Start Recording" }}
                    </button>
                    <button class="btn-secondary" on:click=take_snapshot>
                        "Snapshot"
                    </button>
                </div>
            </div>

            <div class="video-recording-controls">
                <label class="switch-label">
                    <span class="switch">
                        <input type="checkbox" checked=move || auto_recording.get() on:change=toggle_auto_recording/>
                        <span class="slider"></span>
                    </span>
                    "Auto-recording"
                </label>
                <Show when=move || !auto_recording.get()>
                    <span class="recording-disabled-hint">"Manual recording only"</span>
                </Show>
            </div>
            
            <div class="video-wrapper">
                <canvas
                    node_ref=canvas_ref
                    id="video-canvas"
                    width="640"
                    height="480"
                    class="video-canvas"
                />
                <DetectionOverlay detections=detections/>
                <Show when=move || recording.get()>
                    <div class="rec-indicator">
                        <span class="rec-dot"></span>
                        <span class="rec-text">"REC"</span>
                    </div>
                </Show>
                <Show when=move || !stream_active.get()>
                    <div class="stream-placeholder">
                        <span>"Stream inactive"</span>
                        <button class="btn-primary" on:click=move |_| set_stream_active.set(true)>
                            "Start Stream"
                        </button>
                    </div>
                </Show>
            </div>
            
            <div class="video-info">
                <span class="resolution">"640x480"</span>
                <span class="fps">{move || format!("{:.1} FPS", 30.0)}</span>
                <span class="codec">"H.264"</span>
            </div>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn poll_recording_state(
    set_recording: WriteSignal<bool>,
    set_auto_recording: WriteSignal<bool>,
) {
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Ok(resp) = reqwasm::http::Request::get("/api/stats").send().await {
                    if let Ok(stats) = resp.json::<serde_json::Value>().await {
                        if let Some(v) = stats.get("recording").and_then(|v| v.as_bool()) {
                            set_recording.set(v);
                        }
                        if let Some(v) = stats.get("auto_recording").and_then(|v| v.as_bool()) {
                            set_auto_recording.set(v);
                        }
                    }
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
        });
    });
}

#[cfg(target_arch = "wasm32")]
fn manage_websocket(
    stream_active: ReadSignal<bool>,
    set_frame_data: WriteSignal<Option<String>>,
    set_detections: WriteSignal<Vec<DetectionBox>>
) {
    let (_websocket, set_websocket) = signal::<Option<WebSocket>>(None);
    Effect::new(move |_| {
        if stream_active.get() {
            set_websocket.set(Some(connect_websocket(set_frame_data, set_detections)));
        } else {
            set_websocket.update(|ws| {
                if let Some(ws) = ws.take() {
                    let _ = ws.close();
                }
            });
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn connect_websocket(
    set_frame: WriteSignal<Option<String>>,
    set_detections: WriteSignal<Vec<DetectionBox>>
) -> WebSocket {
    use web_sys::MessageEvent;

    let ws_url = web_sys::window()
        .map(|w| {
            let location = w.location();
            let protocol = if location.protocol().unwrap_or_default().starts_with("https") {
                "wss"
            } else {
                "ws"
            };
            format!("{}://{}/ws/stream", protocol, location.host().unwrap_or_else(|_| "localhost:3000".to_string()))
        })
        .unwrap_or_else(|| "ws://localhost:3000/ws/stream".to_string());
    let ws = WebSocket::new(&ws_url).unwrap();
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let onmessage = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(data) = e.data().dyn_into::<js_sys::JsString>() {
            if let Some(json_str) = data.as_string() {
                if let Ok(frame_data) = serde_json::from_str::<FrameMessage>(&json_str) {
                    set_frame.set(Some(frame_data.image));
                    set_detections.set(frame_data.detections);
                }
            }
        }
    }) as Box<dyn FnMut(_)>);

    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
    ws
}

#[cfg(target_arch = "wasm32")]
fn render_frame_to_canvas(
    canvas: &HtmlCanvasElement,
    frame_data: &str,
    detections: Vec<DetectionBox>
) {
    if frame_data.is_empty() {
        return;
    }

    let context = match canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
    {
        Some(c) => c,
        None => return,
    };

    let img = web_sys::HtmlImageElement::new().unwrap();
    let data_url = format!("data:image/jpeg;base64,{}", frame_data);
    img.set_src(&data_url);

    let canvas_width = canvas.width() as f64;
    let canvas_height = canvas.height() as f64;
    let context_clone = context.clone();
    let img_clone = img.clone();
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        let _ = context_clone.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &img_clone, 0.0, 0.0, img_clone.natural_width() as f64, img_clone.natural_height() as f64,
            0.0, 0.0, canvas_width, canvas_height,
        );

        for det in &detections {
            let x = det.x * canvas_width;
            let y = det.y * canvas_height;
            let w = det.width * canvas_width;
            let h = det.height * canvas_height;
            let color = if det.confidence > 0.8 {
                "#22c55e"
            } else if det.confidence > 0.6 {
                "#f59e0b"
            } else {
                "#ef4444"
            };
            context_clone.set_stroke_style_str(color);
            context_clone.set_line_width(3.0);
            let _ = context_clone.stroke_rect(x, y, w, h);
        }
    }) as Box<dyn FnMut()>);

    img.set_onload(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
}


fn take_snapshot(_: leptos::ev::MouseEvent) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(canvas) = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("video-canvas")
        {
            if let Ok(canvas) = canvas.dyn_into::<HtmlCanvasElement>() {
                let _ = canvas.to_data_url_with_type("image/png");
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[cfg(target_arch = "wasm32")]
struct FrameMessage {
    image: String,
    detections: Vec<DetectionBox>,
}
