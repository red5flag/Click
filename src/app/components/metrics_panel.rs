use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn MetricsPanel() -> impl IntoView {
    let (capture_fps, _set_capture_fps) = signal(30.0);
    let (inference_fps, _set_inference_fps) = signal(28.5);
    let (avg_latency, _set_avg_latency) = signal(15.2);
    let (nms_time, _set_nms_time) = signal(0.8);
    
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Ok(resp) = reqwasm::http::Request::get("/api/stats").send().await {
                    if let Ok(stats) = resp.json::<serde_json::Value>().await {
                        if let Some(v) = stats.get("fps").and_then(|v| v.as_f64()) {
                            _set_inference_fps.set(v);
                        }
                        if let Some(v) = stats.get("inference_time").and_then(|v| v.as_f64()) {
                            _set_avg_latency.set(v);
                        }
                    }
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
        });
    });
    
    view! {
        <div class="metrics-panel">
            <h4>"Performance Metrics"</h4>
            <div class="metric-item">
                <span>"Capture FPS"</span>
                <span class="metric-value">{move || format!("{:.1}", capture_fps.get())}</span>
            </div>
            <div class="metric-item">
                <span>"Inference FPS"</span>
                <span class="metric-value">{move || format!("{:.1}", inference_fps.get())}</span>
            </div>
            <div class="metric-item">
                <span>"Avg Latency"</span>
                <span class="metric-value">{move || format!("{:.1}ms", avg_latency.get())}</span>
            </div>
            <div class="metric-item">
                <span>"NMS Time"</span>
                <span class="metric-value">{move || format!("{:.1}ms", nms_time.get())}</span>
            </div>
        </div>
    }
}
