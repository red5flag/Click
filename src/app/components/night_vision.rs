use leptos::prelude::*;
use leptos::{component, view, IntoView};
use wasm_bindgen::JsCast;
use leptos::control_flow::Show;

#[component]
pub fn NightVisionControls() -> impl IntoView {
    let (enabled, set_enabled) = create_signal(false);
    let (mode, set_mode) = create_signal("auto".to_string());
    let (ir_illuminator, set_ir_illuminator) = create_signal(false);
    let (ir_intensity, set_ir_intensity) = create_signal(50_i32);
    let (noise_reduction, set_noise_reduction) = create_signal(true);
    let (noise_level, set_noise_level) = create_signal(50_i32);
    let (gamma_correction, set_gamma_correction) = create_signal(1.8_f64);
    let (digital_zoom, set_digital_zoom) = create_signal(1.0_f64);
    
    let save_settings = move |_| {
        let settings = serde_json::json!({
            "enabled": enabled.get(),
            "mode": mode.get(),
            "ir_illuminator": ir_illuminator.get(),
            "ir_intensity": ir_intensity.get(),
            "noise_reduction": noise_reduction.get(),
            "noise_level": noise_level.get(),
            "gamma_correction": gamma_correction.get(),
            "digital_zoom": digital_zoom.get(),
        });
        
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/camera/settings")
                .header("Content-Type", "application/json")
                .body(settings.to_string())
                .send()
                .await;
        });
    };
    
    view! {
        <div class="night-vision-controls">
            <h3>"Night Vision & Low Light"</h3>
            
            <div class="nv-section">
                <div class="control-row toggle-row">
                    <label>"Enable Night Vision"</label>
                    <label class="switch">
                        <input
                            type="checkbox"
                            checked=move || enabled.get()
                            on:change=move |e| set_enabled.set(event_target_checked(&e))
                        />
                        <span class="slider round"></span>
                    </label>
                </div>
            </div>
            
            <Show when=move || enabled.get()>
                <div class="nv-section">
                    <h4>"Mode"</h4>
                    <div class="mode-selector">
                        <ModeButton label="Auto" mode="auto" icon="🌗" current=mode set=set_mode/>
                        <ModeButton label="IR" mode="ir" icon="📡" current=mode set=set_mode/>
                        <ModeButton label="Thermal" mode="thermal" icon="🌡️" current=mode set=set_mode/>
                        <ModeButton label="Digital" mode="digital" icon="💻" current=mode set=set_mode/>
                    </div>
                </div>
                
                <Show when=move || mode.get() == "ir">
                    <div class="nv-section">
                        <h4>"IR Illuminator"</h4>
                        <Toggle label="Enable IR Light" value=ir_illuminator set_value=set_ir_illuminator/>
                        <Show when=move || ir_illuminator.get()>
                            <Slider label="IR Intensity" value=ir_intensity set_value=set_ir_intensity/>
                        </Show>
                    </div>
                </Show>
                
                <div class="nv-section">
                    <h4>"Image Enhancement"</h4>
                    <Toggle label="Noise Reduction" value=noise_reduction set_value=set_noise_reduction/>
                    <Show when=move || noise_reduction.get()>
                        <Slider label="Reduction Level" value=noise_level set_value=set_noise_level/>
                    </Show>
                    <SliderF label="Gamma Correction" value=gamma_correction set_value=set_gamma_correction min=0.5 max=3.0 step=0.1/>
                    <SliderF label="Digital Zoom" value=digital_zoom set_value=set_digital_zoom min=1.0 max=4.0 step=0.1/>
                </div>
            </Show>
            
            <div class="nv-actions">
                <button class="btn-primary" on:click=save_settings>
                    "Apply Settings"
                </button>
            </div>
        </div>
    }
}

#[component]
fn ModeButton(label: &'static str, mode: &'static str, icon: &'static str, current: ReadSignal<String>, set: WriteSignal<String>) -> impl IntoView {
    let is_active = move || current() == mode;
    let mode = mode.to_string();
    
    view! {
        <button
            class={move || if is_active() { "mode-btn active" } else { "mode-btn" }}
            on:click=move |_| set(mode.clone())
        >
            <span class="mode-icon">{icon}</span>
            <span>{label}</span>
        </button>
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
fn SliderF(label: &'static str, value: ReadSignal<f64>, set_value: WriteSignal<f64>, min: f64, max: f64, step: f64) -> impl IntoView {
    view! {
        <div class="control-row">
            <label>{label}</label>
            <input
                type="range"
                min=min
                max=max
                step=step
                value=move || value.get()
                on:input=move |e| set_value.set(event_target_value(&e).parse().unwrap_or(value.get()))
            />
            <span>{move || format!("{:.1}", value.get())}</span>
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
    event
        .target()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value()
}

fn event_target_checked(event: &web_sys::Event) -> bool {
    event
        .target()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .checked()
}
