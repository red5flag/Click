use leptos::prelude::*;
use leptos::{component, view, IntoView};
use wasm_bindgen::JsCast;
use leptos::control_flow::Show;

#[component]
pub fn ColorCorrection() -> impl IntoView {
    let (enabled, set_enabled) = signal(false);
    let (preset, set_preset) = signal("natural".to_string());
    let (temperature, set_temperature) = signal(6500_i32);
    let (tint, set_tint) = signal(0_i32);
    let (hue_shift, set_hue_shift) = signal(0_i32);
    let (vibrance, set_vibrance) = signal(0_i32);
    let (shadow_boost, set_shadow_boost) = signal(0_i32);
    let (vignette, set_vignette) = signal(0_i32);
    let (dehaze, set_dehaze) = signal(0_i32);
    
    let presets = vec![
        ("natural", "Natural", "🌿"),
        ("vivid", "Vivid", "🎨"),
        ("portrait", "Portrait", "👤"),
        ("landscape", "Landscape", "🏞️"),
        ("night", "Night", "🌙"),
        ("indoor", "Indoor", "🏠"),
        ("outdoor", "Outdoor", "☀️"),
        ("monochrome", "Monochrome", "⬛"),
        ("sepia", "Sepia", "📜"),
    ];
    
    let save_settings = move |_| {
        let _settings = serde_json::json!({
            "enabled": enabled.get(),
            "preset": preset.get(),
            "temperature": temperature.get(),
            "tint": tint.get(),
            "hue_shift": hue_shift.get(),
            "vibrance": vibrance.get(),
            "shadow_boost": shadow_boost.get(),
            "vignette": vignette.get(),
            "dehaze": dehaze.get(),
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
        <div class="color-correction">
            <h3>"Color Correction"</h3>
            
            <div class="cc-section">
                <div class="control-row toggle-row">
                    <label>"Enable Color Correction"</label>
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
                <div class="cc-section">
                    <h4>"Presets"</h4>
                    <div class="preset-grid">
                        {presets.iter().map(|(id, name, icon)| {
                            let (id, name, icon) = (*id, *name, *icon);
                            let id_clone = id.to_string();
                            let is_active = move || preset.get() == id;
                            view! {
                                <button
                                    class={move || if is_active() { "preset-btn active" } else { "preset-btn" }}
                                    on:click=move |_| set_preset.set(id_clone.clone())
                                >
                                    <span class="preset-icon">{icon}</span>
                                    <span class="preset-name">{name}</span>
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
                
                <div class="cc-section">
                    <h4>"White Balance"</h4>
                    <SliderRange label="Temperature (K)" value=temperature set_value=set_temperature min=2000 max=10000 step=100/>
                    <SliderRange label="Tint" value=tint set_value=set_tint min=-50 max=50 step=1/>
                </div>
                
                <div class="cc-section">
                    <h4>"Tonal Adjustments"</h4>
                    <SliderRange label="Hue Shift" value=hue_shift set_value=set_hue_shift min=-30 max=30 step=1/>
                    <SliderRange label="Vibrance" value=vibrance set_value=set_vibrance min=-50 max=50 step=1/>
                    <SliderRange label="Shadow Boost" value=shadow_boost set_value=set_shadow_boost min=0 max=100 step=1/>
                </div>
                
                <div class="cc-section">
                    <h4>"Creative Effects"</h4>
                    <SliderRange label="Vignette" value=vignette set_value=set_vignette min=0 max=100 step=1/>
                    <SliderRange label="Dehaze" value=dehaze set_value=set_dehaze min=0 max=100 step=1/>
                </div>
            </Show>
            
            <div class="cc-actions">
                <button class="btn-primary" on:click=save_settings>
                    "Apply Settings"
                </button>
            </div>
        </div>
    }
}

#[component]
fn SliderRange(label: &'static str, value: ReadSignal<i32>, set_value: WriteSignal<i32>, min: i32, max: i32, step: i32) -> impl IntoView {
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
            <span>{move || value.get().to_string()}</span>
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
