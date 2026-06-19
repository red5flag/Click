use leptos::prelude::*;
use leptos::{component, view, IntoView};
use crate::app::components::*;

#[component]
pub fn TagsPage() -> impl IntoView {
    view! {
        <div class="tags-page">
            <h1>"Tag Management"</h1>
            
            <div class="tags-layout">
                <div class="tags-main">
                    <TagManager/>
                </div>
                <div class="tags-sidebar">
                    <div class="tag-help">
                        <h4>"About Tags"</h4>
                        <p>"Tags help you organize and filter detections."</p>
                    </div>
                </div>
            </div>
        </div>
    }
}
