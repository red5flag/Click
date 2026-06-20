use leptos::prelude::*;
use leptos::{component, view, IntoView};
use crate::app::components::*;

#[component]
pub fn Dashboard() -> impl IntoView {
    let (persons_detected, _set_persons_detected) = signal(0);
    let (fps, _set_fps) = signal(30.0);
    let (recording, _set_recording) = signal(false);
    
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Ok(resp) = reqwasm::http::Request::get("/api/stats").send().await {
                    if let Ok(stats) = resp.json::<serde_json::Value>().await {
                        if let Some(v) = stats.get("persons_detected").and_then(|v| v.as_u64()) {
                            _set_persons_detected.set(v as usize);
                        }
                        if let Some(v) = stats.get("fps").and_then(|v| v.as_f64()) {
                            _set_fps.set(v);
                        }
                        if let Some(v) = stats.get("recording").and_then(|v| v.as_bool()) {
                            _set_recording.set(v);
                        }
                    }
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
        });
    });
    
    let toggle_recording = move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/recording/toggle").send().await;
        });
    };
    
    view! {
        <div class="dashboard">
            <div class="dashboard-header">
                <h1>"Dashboard"</h1>
                <button
                    class={move || if recording.get() { "btn-stop" } else { "btn-primary" }}
                    on:click=toggle_recording
                >
                    {move || if recording.get() { "Stop Recording" } else { "Start Recording" }}
                </button>
            </div>
            
            <div class="stats-grid">
                <div class="stat-card">
                    <span class="stat-icon">"👁️"</span>
                    <div class="stat-content">
                        <span class="stat-value">{move || persons_detected.get()}</span>
                        <span class="stat-label">"Persons Detected"</span>
                    </div>
                </div>
                <div class="stat-card">
                    <span class="stat-icon">"🎥"</span>
                    <div class="stat-content">
                        <span class="stat-value">{move || format!("{:.1}", fps.get())}</span>
                        <span class="stat-label">"FPS"</span>
                    </div>
                </div>
            </div>
            
            <div class="dashboard-content">
                <div class="main-panel">
                    <VideoFeed/>
                </div>
                <div class="side-panel">
                    <MetricsPanel/>
                    <RecordingIndicator/>
                </div>
            </div>
        </div>
    }
}
