pub mod app;

#[cfg(not(target_arch = "wasm32"))]
pub mod camera;
#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod detection;
#[cfg(not(target_arch = "wasm32"))]
pub mod metrics;
#[cfg(not(target_arch = "wasm32"))]
pub mod model;
#[cfg(not(target_arch = "wasm32"))]
pub mod recording;
#[cfg(not(target_arch = "wasm32"))]
pub mod types;
#[cfg(not(target_arch = "wasm32"))]
pub mod web;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(feature = "hydrate")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "hydrate")]
#[wasm_bindgen]
pub fn hydrate() {
    use app::App;
    use leptos::prelude::*;
    use leptos::mount::mount_to_body;
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> });
}
