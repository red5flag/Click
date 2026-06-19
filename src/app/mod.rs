use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_meta::*;
use leptos_router::components::{Router, Routes, Route, A};
use leptos_router::path;

#[cfg(not(target_arch = "wasm32"))]
pub mod application;
pub mod components;
pub mod pages;

use components::*;
use pages::*;

#[cfg(not(target_arch = "wasm32"))]
pub use application::{Application, ShutdownHandle};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    view! {
        <Router>
            <nav class="navbar">
                <div class="nav-brand">"PersonDetect"</div>
                <div class="nav-links">
                    <A href="/" exact=true>"Dashboard"</A>
                    <A href="/camera">"Camera"</A>
                    <A href="/models">"Models"</A>
                    <A href="/tags">"Tags"</A>
                    <A href="/settings">"Settings"</A>
                </div>
                <div class="nav-status">
                    <ConnectionStatus/>
                </div>
            </nav>
            
            <main class="main-content">
                <Routes fallback=|| view! { <NotFound/> }>
                    <Route path=path!("/") view=Dashboard/>
                    <Route path=path!("/camera") view=CameraPage/>
                    <Route path=path!("/models") view=ModelsPage/>
                    <Route path=path!("/tags") view=TagsPage/>
                    <Route path=path!("/settings") view=SettingsPage/>
                    <Route path=path!("/*any") view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn ConnectionStatus() -> impl IntoView {
    let (connected, _set_connected) = create_signal(true);
    
    view! {
        <span class={move || if connected.get() { "status-online" } else { "status-offline" }}>
            {move || if connected.get() { "● Connected" } else { "● Disconnected" }}
        </span>
    }
}

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="not-found">
            <h1>"404"</h1>
            <p>"Page not found"</p>
        </div>
    }
}

#[component]
pub fn Loading() -> impl IntoView {
    view! {
        <div class="loading">
            <div class="spinner"></div>
            <span>"Loading..."</span>
        </div>
    }
}
