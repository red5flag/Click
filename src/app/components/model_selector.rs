use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::{Show, For};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub path: String,
    pub name: String,
    pub size_mb: f64,
    pub model_type: String,
    pub input_shape: String,
    pub num_classes: i32,
    pub version: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[cfg(target_arch = "wasm32")]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[component]
pub fn ModelSelector() -> impl IntoView {
    let (models, _set_models) = signal::<Vec<ModelInfo>>(vec![]);
    let (selected_model, set_selected_model) = signal::<Option<String>>(None);
    let (model_loading, set_model_loading) = signal(false);
    
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(resp) = reqwasm::http::Request::get("/api/models").send().await {
                if let Ok(api) = resp.json::<ApiResponse<Vec<ModelInfo>>>().await {
                    if let Some(fetched) = api.data {
                        _set_models.set(fetched);
                    }
                }
            }
        });
    });
    
    let select_model = move |model_path: String| {
        set_model_loading.set(true);
        set_selected_model.set(Some(model_path.clone()));
        
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::post("/api/models/load")
                .header("Content-Type", "application/json")
                .body(serde_json::json!({"path": model_path}).to_string())
                .send()
                .await;
            set_model_loading.set(false);
        });
    };
    
    view! {
        <div class="model-selector">
            <h3>"Model Selection"</h3>
            
            <div class="model-list">
                <For
                    each=move || models.get()
                    key=|m| m.path.clone()
                    children=move |model| {
                        let path = model.path.clone();
                        let path_for_click = path.clone();
                        let path_for_show = path.clone();
                        
                        view! {
                            <div
                                class={move || if selected_model.get().as_ref() == Some(&path) { "model-card selected" } else { "model-card" }}
                                on:click=move |_| select_model(path_for_click.clone())
                            >
                                <div class="model-header">
                                    <h4>{model.name.clone()}</h4>
                                    <Show when=move || selected_model.get().as_ref() == Some(&path_for_show)>
                                        <span class="model-badge">"Active"</span>
                                    </Show>
                                </div>
                                <div class="model-details">
                                    <span>{format!("{:.1} MB", model.size_mb)}</span>
                                    <span>{model.input_shape.clone()}</span>
                                    <span>{format!("{} classes", model.num_classes)}</span>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
            
            <Show when=move || model_loading.get()>
                <div class="model-loading">
                    <div class="spinner"></div>
                    <span>"Loading model..."</span>
                </div>
            </Show>
        </div>
    }
}
