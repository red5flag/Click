use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::Show;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let (active_section, set_active_section) = signal("general");
    
    view! {
        <div class="settings-page">
            <h1>"System Settings"</h1>
            
            <div class="settings-layout">
                <div class="settings-nav">
                    <NavItem label="General" section="general" active=active_section set=set_active_section/>
                    <NavItem label="Storage" section="storage" active=active_section set=set_active_section/>
                    <NavItem label="Network" section="network" active=active_section set=set_active_section/>
                    <NavItem label="Notifications" section="notifications" active=active_section set=set_active_section/>
                    <NavItem label="Advanced" section="advanced" active=active_section set=set_active_section/>
                </div>
                
                <div class="settings-content">
                    <Show when=move || active_section.get() == "general">
                        <GeneralSettings/>
                    </Show>
                    <Show when=move || active_section.get() == "storage">
                        <StorageSettings/>
                    </Show>
                    <Show when=move || active_section.get() == "network">
                        <NetworkSettings/>
                    </Show>
                    <Show when=move || active_section.get() == "notifications">
                        <NotificationSettings/>
                    </Show>
                    <Show when=move || active_section.get() == "advanced">
                        <AdvancedSettings/>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
fn NavItem(label: &'static str, section: &'static str, active: ReadSignal<&'static str>, set: WriteSignal<&'static str>) -> impl IntoView {
    let is_active = move || active.get() == section;
    
    view! {
        <button
            class={move || if is_active() { "nav-item active" } else { "nav-item" }}
            on:click=move |_| set.set(section)
        >
            {label}
        </button>
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
