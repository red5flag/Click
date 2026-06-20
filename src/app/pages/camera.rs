use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::Show;
use crate::app::components::*;

#[component]
pub fn CameraPage() -> impl IntoView {
    let (active_tab, set_active_tab) = signal("settings");
    
    view! {
        <div class="camera-page">
            <h1>"Camera Configuration"</h1>
            
            <div class="tab-nav">
                <TabButton label="Basic Settings" tab="settings" active=active_tab set=set_active_tab/>
                <TabButton label="Night Vision" tab="night" active=active_tab set=set_active_tab/>
                <TabButton label="Color Correction" tab="color" active=active_tab set=set_active_tab/>
                <TabButton label="Detection Tuning" tab="detection" active=active_tab set=set_active_tab/>
            </div>
            
            <div class="tab-content">
                <Show when=move || active_tab.get() == "settings">
                    <div class="panel-grid">
                        <CameraControls/>
                        <VideoFeed/>
                    </div>
                </Show>
                <Show when=move || active_tab.get() == "night">
                    <div class="panel-grid">
                        <NightVisionControls/>
                        <VideoFeed/>
                    </div>
                </Show>
                <Show when=move || active_tab.get() == "color">
                    <div class="panel-grid">
                        <ColorCorrection/>
                        <VideoFeed/>
                    </div>
                </Show>
                <Show when=move || active_tab.get() == "detection">
                    <div class="panel-grid">
                        <ToleranceControls/>
                        <VideoFeed/>
                    </div>
                </Show>
            </div>
        </div>
    }
}

#[component]
fn TabButton(label: &'static str, tab: &'static str, active: ReadSignal<&'static str>, set: WriteSignal<&'static str>) -> impl IntoView {
    let is_active = move || active.get() == tab;
    
    view! {
        <button
            class={move || if is_active() { "tab-btn active" } else { "tab-btn" }}
            on:click=move |_| set.set(tab)
        >
            {label}
        </button>
    }
}
