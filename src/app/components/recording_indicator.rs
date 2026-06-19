use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::Show;

#[component]
pub fn RecordingIndicator() -> impl IntoView {
    let (recording, set_recording) = create_signal(false);
    let (elapsed, set_elapsed) = create_signal(0_u64);
    
    create_effect(move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Ok(resp) = reqwasm::http::Request::get("/api/stats").send().await {
                    if let Ok(stats) = resp.json::<serde_json::Value>().await {
                        if let Some(rec) = stats.get("recording").and_then(|v| v.as_bool()) {
                            set_recording.set(rec);
                        }
                    }
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
        });
    });
    
    create_effect(move |_| {
        if recording.get() {
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                loop {
                    if !recording.get() { break; }
                    set_elapsed.update(|e| *e += 1);
                    gloo_timers::future::TimeoutFuture::new(1000).await;
                }
            });
        } else {
            set_elapsed.set(0);
        }
    });
    
    view! {
        <div class={move || if recording.get() { "recording-indicator active" } else { "recording-indicator" }}>
            <Show when=move || recording.get()>
                <span class="recording-dot"></span>
                <span>"REC "</span>
                <span class="recording-time">{move || format_elapsed(elapsed.get())}</span>
            </Show>
            <Show when=move || !recording.get()>
                <span>"Not Recording"</span>
            </Show>
        </div>
    }
}

fn format_elapsed(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}
