use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::{Show, For};
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub usage_count: i32,
    pub category: String,
}

#[component]
pub fn TagManager() -> impl IntoView {
    let (tags, set_tags) = create_signal::<Vec<Tag>>(vec![]);
    let (new_tag_name, set_new_tag_name) = create_signal(String::new());
    let (new_tag_color, set_new_tag_color) = create_signal("#3b82f6".to_string());
    let (search_query, set_search_query) = create_signal(String::new());
    let (show_form, set_show_form) = create_signal(false);
    
    create_effect(move |_| {
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(resp) = reqwasm::http::Request::get("/api/tags").send().await {
                if let Ok(loaded) = resp.json::<Vec<Tag>>().await {
                    set_tags.set(loaded);
                }
            }
        });
    });
    
    let filtered_tags = move || {
        let query = search_query.get().to_lowercase();
        tags.get().into_iter()
            .filter(|t| t.name.to_lowercase().contains(&query))
            .collect::<Vec<_>>()
    };
    
    let add_tag = move |_| {
        let name = new_tag_name.get();
        if !name.is_empty() {
            let tag = Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.clone(),
                color: new_tag_color.get(),
                created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                usage_count: 0,
                category: "General".to_string(),
            };
            set_tags.update(|t| t.push(tag.clone()));
            
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                let _ = reqwasm::http::Request::post("/api/tags")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&tag).unwrap())
                    .send()
                    .await;
            });
            
            set_new_tag_name.set(String::new());
            set_show_form.set(false);
        }
    };
    
    let delete_tag = move |id: String| {
        set_tags.update(|t| t.retain(|tag| tag.id != id));
        
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = reqwasm::http::Request::delete(&format!("/api/tags/{}", id))
                .send()
                .await;
        });
    };
    
    let colors = vec![
        "#ef4444", "#f97316", "#f59e0b", "#84cc16",
        "#22c55e", "#14b8a6", "#06b6d4", "#0ea5e9",
        "#3b82f6", "#8b5cf6", "#a855f7", "#d946ef",
        "#ec4899", "#f43f5e", "#64748b", "#94a3b8"
    ];
    
    view! {
        <div class="tag-manager">
            <h3>"Tag Management"</h3>
            
            <div class="tag-toolbar">
                <input
                    type="text"
                    placeholder="Search tags..."
                    value=move || search_query.get()
                    on:input=move |e| set_search_query.set(event_target_value(&e))
                />
                <button class="btn-primary" on:click=move |_| set_show_form.set(true)>
                    "+ New Tag"
                </button>
            </div>
            
            <Show when=move || show_form.get()>
                <div class="tag-form">
                    <h4>"Create New Tag"</h4>
                    <input
                        type="text"
                        placeholder="Enter tag name..."
                        value=move || new_tag_name.get()
                        on:input=move |e| set_new_tag_name.set(event_target_value(&e))
                    />
                    <div class="color-picker">
                        {colors.into_iter().map(|color| {
                            let color_clone = color.to_string();
                            let is_selected = move || new_tag_color.get() == color;
                            view! {
                                <button
                                    class={move || if is_selected() { "color-swatch active" } else { "color-swatch" }}
                                    style=format!("background-color: {}", color)
                                    on:click=move |_| set_new_tag_color.set(color_clone.clone())
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                    <div class="form-actions">
                        <button class="btn-primary" on:click=add_tag>
                            "Create Tag"
                        </button>
                        <button class="btn-secondary" on:click=move |_| set_show_form.set(false)>
                            "Cancel"
                        </button>
                    </div>
                </div>
            </Show>
            
            <div class="tags-grid">
                <For
                    each=filtered_tags
                    key=|t| t.id.clone()
                    children=move |tag| {
                        let tag_id = tag.id.clone();
                        view! {
                            <div class="tag-card">
                                <div
                                    class="tag-color-bar"
                                    style=format!("background-color: {}", tag.color)
                                />
                                <div class="tag-content">
                                    <span class="tag-name">{tag.name.clone()}</span>
                                    <span class="tag-count">{format!("{} uses", tag.usage_count)}</span>
                                </div>
                                <button
                                    class="tag-delete"
                                    on:click=move |_| delete_tag(tag_id.clone())
                                >
                                    "×"
                                </button>
                            </div>
                        }
                    }
                />
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
