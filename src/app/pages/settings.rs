use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::Show;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div class="settings-page">
            <h1>"System Settings"</h1>
            <div class="settings-content">
                <GeneralSettings/>
                <StorageSettings/>
                <NetworkSettings/>
                <NotificationSettings/>
                <TestingSection/>
                <AdvancedSettings/>
            </div>
        </div>
    }
}

#[component]
fn GeneralSettings() -> impl IntoView {
    view! {
        <div class="settings-section">
            <h3>"General Settings"</h3>
            <div class="setting-item"><label>"Application Name"</label><input type="text" value="PersonDetect"/></div>
            <div class="setting-item"><label>"Theme"</label><select><option>"Dark"</option><option>"Light"</option></select></div>
        </div>
    }
}

#[component]
fn StorageSettings() -> impl IntoView {
    view! {
        <div class="settings-section">
            <h3>"Storage Settings"</h3>
            <div class="setting-item"><label>"Recordings Directory"</label><input type="text" value="./recordings"/></div>
            <div class="setting-item"><label>"Max Storage (GB)"</label><input type="number" value="100"/></div>
        </div>
    }
}

#[component]
fn NetworkSettings() -> impl IntoView {
    view! {
        <div class="settings-section">
            <h3>"Network Settings"</h3>
            <div class="setting-item"><label>"Web Interface Port"</label><input type="number" value="3000"/></div>
            <div class="setting-item"><label>"Allow remote access"</label><input type="checkbox"/></div>
        </div>
    }
}

#[component]
fn NotificationSettings() -> impl IntoView {
    view! {
        <div class="settings-section">
            <h3>"Notification Settings"</h3>
            <div class="setting-item"><label>"Enable notifications"</label><input type="checkbox" checked/></div>
            <div class="setting-item"><label>"Notify on detection"</label><input type="checkbox" checked/></div>
        </div>
    }
}

#[component]
fn AdvancedSettings() -> impl IntoView {
    view! {
        <div class="settings-section">
            <h3>"Advanced Settings"</h3>
            <div class="setting-item"><label>"Debug logging"</label><input type="checkbox"/></div>
            <div class="setting-item"><label>"GPU acceleration"</label><input type="checkbox"/></div>
            <div class="danger-zone">
                <h4>"Danger Zone"</h4>
                <button class="btn-danger">"Reset All Settings"</button>
            </div>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct DetectionSettings {
    confidence_threshold: f64,
    iou_threshold: f64,
    grace_period_seconds: i32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct SystemStats {
    persons_detected: usize,
    fps: f64,
    inference_time: f64,
    recording: bool,
    storage_used: f64,
}

#[component]
fn TestingSection() -> impl IntoView {
    #[allow(unused_variables)]
    let (threshold, set_threshold) = signal(0.8f64);
    #[allow(unused_variables)]
    let (saved, set_saved) = signal(false);
    #[allow(unused_variables)]
    let (stats, set_stats) = signal(SystemStats::default());

    #[cfg(target_arch = "wasm32")]
    {
        // Load current detection settings
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(resp) = reqwasm::http::Request::get("/api/detection/settings").send().await {
                if let Ok(api) = resp.json::<ApiResponse<DetectionSettings>>().await {
                    if let Some(s) = api.data {
                        set_threshold.set(s.confidence_threshold);
                    }
                }
            }
        });

        // Poll stats every second
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                if let Ok(resp) = reqwasm::http::Request::get("/api/stats").send().await {
                    if let Ok(api) = resp.json::<ApiResponse<SystemStats>>().await {
                        if let Some(s) = api.data {
                            set_stats.set(s);
                        }
                    }
                }
                gloo_timers::future::sleep(std::time::Duration::from_millis(1000)).await;
            }
        });
    }

    let update_threshold = move |_ev: leptos::ev::Event| {
        #[cfg(target_arch = "wasm32")]
        {
            let input = _ev.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
            let value: f64 = input.value().parse().unwrap_or(0.8);
            let value = value.clamp(0.0, 1.0);
            set_threshold.set(value);
            let settings = DetectionSettings {
                confidence_threshold: value,
                iou_threshold: 0.45,
                grace_period_seconds: 10,
            };
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(_) = reqwasm::http::Request::post("/api/detection/settings")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&settings).unwrap_or_default())
                    .send()
                    .await
                {
                    set_saved.set(true);
                    gloo_timers::future::sleep(std::time::Duration::from_millis(1500)).await;
                    set_saved.set(false);
                }
            });
        }
    };

    let toggle_recording = move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/recording/toggle").send().await;
        });
    };

    view! {
        <div class="settings-section">
            <h3>"Detection Testing"</h3>
            <p>"Adjust the detection threshold and verify that recording starts when a person is detected."</p>

            <div class="setting-item">
                <label>"Confidence Threshold (0.0 - 1.0)"</label>
                <input
                    type="number"
                    min="0.0"
                    max="1.0"
                    step="0.05"
                    value=move || format!("{:.2}", threshold.get())
                    on:change=update_threshold
                />
            </div>

            <div class="setting-item">
                <label>"Current Threshold"</label>
                <span>{move || format!("{:.0}%", threshold.get() * 100.0)}</span>
            </div>

            <Show when=move || saved.get()>
                <div class="setting-saved">"Threshold saved"</div>
            </Show>

            <div class="test-stats">
                <div class="stat"><strong>"Persons detected:"</strong> {move || stats.get().persons_detected}</div>
                <div class="stat"><strong>"FPS:"</strong> {move || format!("{:.1}", stats.get().fps)}</div>
                <div class="stat"><strong>"Inference:"</strong> {move || format!("{:.1} ms", stats.get().inference_time)}</div>
                <div class="stat"><strong>"Recording:"</strong> {move || if stats.get().recording { "Active" } else { "Idle" }}</div>
            </div>

            <div class="setting-item">
                <button class="btn-primary" on:click=toggle_recording>"Toggle Manual Recording"</button>
            </div>

            <p class="hint">"Default threshold is 80%. Lower it temporarily to see all raw YOLO boxes on the video feed."</p>
        </div>
    }
}
