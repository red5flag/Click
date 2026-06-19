use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::Show;

#[cfg(target_arch = "wasm32")]
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};

use super::detection_overlay::{DetectionBox, DetectionOverlay};

#[component]
pub fn VideoFeed() -> impl IntoView {
    let (stream_active, set_stream_active) = create_signal(false);
    let (frame_data, set_frame_data) = create_signal::<Option<String>>(None);
    let (detections, set_detections) = create_signal::<Vec<DetectionBox>>(vec![]);
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();
    
    create_effect(move |_| {
        if stream_active.get() {
            connect_websocket(set_frame_data, set_detections);
        }
    });
    
    create_effect(move |_| {
        if let Some(data) = frame_data.get() {
            if let Some(canvas) = canvas_ref.get() {
                #[cfg(target_arch = "wasm32")]
                {
                    let canvas_el = canvas.unchecked_ref::<HtmlCanvasElement>();
                    render_frame_to_canvas(canvas_el, &data, detections.get());
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    render_frame_to_canvas(&canvas, &data, detections.get());
                }
            }
        }
    });
    
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
                    <button class="btn-secondary" on:click=take_snapshot>
                        "Snapshot"
                    </button>
                </div>
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

fn connect_websocket(
    set_frame: WriteSignal<Option<String>>,
    set_detections: WriteSignal<Vec<DetectionBox>>
) {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::{WebSocket, MessageEvent};
        
        let ws = WebSocket::new("ws://localhost:3000/ws/stream").unwrap();
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        
        let onmessage = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(data) = e.data().dyn_into::<js_sys::JsString>() {
                if let Some(json_str) = data.as_string() {
                    if let Ok(frame_data) = serde_json::from_str::<FrameMessage>(&json_str) {
                        set_frame(Some(frame_data.image));
                        set_detections.set(frame_data.detections);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    }
}

#[cfg(target_arch = "wasm32")]
fn render_frame_to_canvas(
    canvas: &HtmlCanvasElement,
    _frame_data: &str,
    _detections: Vec<DetectionBox>
) {
    if let Ok(context) = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
    {
        let _ = context;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn render_frame_to_canvas(
    _canvas: &leptos::html::Canvas,
    _frame_data: &str,
    _detections: Vec<DetectionBox>
) {
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
struct FrameMessage {
    image: String,
    detections: Vec<DetectionBox>,
}
