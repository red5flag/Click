use leptos::prelude::*;
use leptos::{component, view, IntoView};
use wasm_bindgen::JsCast;

#[component]
pub fn ToleranceControls() -> impl IntoView {
    let (confidence_threshold, set_confidence_threshold) = signal(0.5_f64);
    let (iou_threshold, set_iou_threshold) = signal(0.45_f64);
    let (grace_period, set_grace_period) = signal(10_i32);
    
    let save_settings = move |_| {
        let _settings = serde_json::json!({
            "confidence_threshold": confidence_threshold.get(),
            "iou_threshold": iou_threshold.get(),
            "grace_period_seconds": grace_period.get(),
        });
        
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/detection/settings")
                .header("Content-Type", "application/json")
                .body(_settings.to_string())
                .send()
                .await;
        });
    };
    
    view! {
        <div class="tolerance-controls">
            <h3>"Detection Tolerance"</h3>
            
            <div class="tolerance-section">
                <h4>"Confidence Threshold"</h4>
                <input
                    type="range"
                    min="0.0"
                    max="1.0"
                    step="0.01"
                    value=move || confidence_threshold.get()
                    on:input=move |e| set_confidence_threshold.set(event_target_value(&e).parse().unwrap_or(0.5))
                />
                <span>{move || format!("{:.0}%", confidence_threshold.get() * 100.0)}</span>
                <p class="tolerance-description">
                    "Minimum confidence required to consider a detection valid."
                </p>
            </div>
            
            <div class="tolerance-section">
                <h4>"IoU Threshold (NMS)"</h4>
                <input
                    type="range"
                    min="0.0"
                    max="1.0"
                    step="0.01"
                    value=move || iou_threshold.get()
                    on:input=move |e| set_iou_threshold.set(event_target_value(&e).parse().unwrap_or(0.45))
                />
                <span>{move || format!("{:.0}%", iou_threshold.get() * 100.0)}</span>
            </div>
            
            <div class="tolerance-section">
                <h4>"Grace Period"</h4>
                <input
                    type="range"
                    min="0"
                    max="60"
                    value=move || grace_period.get()
                    on:input=move |e| set_grace_period.set(event_target_value(&e).parse().unwrap_or(10))
                />
                <span>{move || format!("{}s", grace_period.get())}</span>
            </div>
            
            <div class="tolerance-actions">
                <button class="btn-primary" on:click=save_settings>
                    "Save Settings"
                </button>
            </div>
        </div>
    }
}

fn event_target_value(event: &web_sys::Event) -> String {
    event
        .target()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value()
}
