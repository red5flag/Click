use leptos::prelude::*;
use leptos::{component, view, IntoView};
use wasm_bindgen::JsCast;

#[component]
pub fn CameraControls() -> impl IntoView {
    let (camera_index, set_camera_index) = signal(0_i32);
    let (resolution, set_resolution) = signal("640x480".to_string());
    let (fps, set_fps) = signal(30_i32);
    let (brightness, set_brightness) = signal(50_i32);
    let (contrast, set_contrast) = signal(50_i32);
    let (saturation, set_saturation) = signal(50_i32);
    let (auto_exposure, set_auto_exposure) = signal(true);
    let (auto_white_balance, set_auto_white_balance) = signal(true);
    let (auto_gain, set_auto_gain) = signal(true);
    
    let save_settings = move |_| {
        let _settings = serde_json::json!({
            "camera_index": camera_index.get(),
            "resolution": resolution.get(),
            "fps": fps.get(),
            "brightness": brightness.get(),
            "contrast": contrast.get(),
            "saturation": saturation.get(),
            "auto_exposure": auto_exposure.get(),
            "auto_white_balance": auto_white_balance.get(),
            "auto_gain": auto_gain.get(),
        });
        
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/camera/settings")
                .header("Content-Type", "application/json")
                .body(_settings.to_string())
                .send()
                .await;
        });
    };
    
    view! {
        <div class="camera-controls">
            <h3>"Camera Settings"</h3>
            
            <div class="control-section">
                <h4>"Device"</h4>
                <div class="control-row">
                    <label>"Camera Index"</label>
                    <select
                        prop:value=move || camera_index.get().to_string()
                        on:change=move |e| set_camera_index.set(event_target_value(&e).parse().unwrap_or(0))
                    >
                        <option value="0">"Camera 0"</option>
                        <option value="1">"Camera 1"</option>
                        <option value="2">"Camera 2"</option>
                    </select>
                </div>
                
                <div class="control-row">
                    <label>"Resolution"</label>
                    <select
                        prop:value=move || resolution.get()
                        on:change=move |e| set_resolution.set(event_target_value(&e))
                    >
                        <option value="320x240">"320x240"</option>
                        <option value="640x480">"640x480"</option>
                        <option value="1280x720">"1280x720"</option>
                        <option value="1920x1080">"1920x1080"</option>
                    </select>
                </div>
                
                <div class="control-row">
                    <label>"FPS Target"</label>
                    <input
                        type="range"
                        min="5"
                        max="60"
                        value=move || fps.get()
                        on:input=move |e| set_fps.set(event_target_value(&e).parse().unwrap_or(30))
                    />
                    <span>{move || format!("{} fps", fps.get())}</span>
                </div>
            </div>
            
            <div class="control-section">
                <h4>"Image Adjustment"</h4>
                <Slider label="Brightness" value=brightness set_value=set_brightness/>
                <Slider label="Contrast" value=contrast set_value=set_contrast/>
                <Slider label="Saturation" value=saturation set_value=set_saturation/>
            </div>
            
            <div class="control-section">
                <h4>"Auto Controls"</h4>
                <Toggle label="Auto Exposure" value=auto_exposure set_value=set_auto_exposure/>
                <Toggle label="Auto White Balance" value=auto_white_balance set_value=set_auto_white_balance/>
                <Toggle label="Auto Gain" value=auto_gain set_value=set_auto_gain/>
            </div>
            
            <div class="control-actions">
                <button class="btn-primary" on:click=save_settings>
                    "Apply Settings"
                </button>
            </div>
        </div>
    }
}

#[component]
fn Slider(label: &'static str, value: ReadSignal<i32>, set_value: WriteSignal<i32>) -> impl IntoView {
    view! {
        <div class="control-row">
            <label>{label}</label>
            <input
                type="range"
                min="0"
                max="100"
                value=move || value.get()
                on:input=move |e| set_value.set(event_target_value(&e).parse().unwrap_or(50))
            />
            <span>{move || value.get().to_string()}</span>
        </div>
    }
}

#[component]
fn Toggle(label: &'static str, value: ReadSignal<bool>, set_value: WriteSignal<bool>) -> impl IntoView {
    view! {
        <div class="control-row toggle-row">
            <label>{label}</label>
            <input
                type="checkbox"
                checked=move || value.get()
                on:change=move |e| set_value.set(event_target_checked(&e))
            />
        </div>
    }
}

fn event_target_value(event: &web_sys::Event) -> String {
    if let Some(target) = event.target() {
        if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
            return input.value();
        }
        if let Some(select) = target.dyn_ref::<web_sys::HtmlSelectElement>() {
            return select.value();
        }
    }
    String::new()
}

fn event_target_checked(event: &web_sys::Event) -> bool {
    if let Some(target) = event.target() {
        if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
            return input.checked();
        }
    }
    false
}
