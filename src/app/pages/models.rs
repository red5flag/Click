use leptos::prelude::*;
use leptos::{component, view, IntoView};
use crate::app::components::*;

#[component]
pub fn ModelsPage() -> impl IntoView {
    view! {
        <div class="models-page">
            <h1>"Model Management"</h1>
            
            <div class="models-layout">
                <div class="models-sidebar">
                    <ModelSelector/>
                </div>
                <div class="models-main">
                    <div class="model-info-card">
                        <h3>"Active Model"</h3>
                        <div class="model-details">
                            <div class="detail-row">
                                <span class="detail-label">"Name:"</span>
                                <span class="detail-value">"YOLO12n"</span>
                            </div>
                            <div class="detail-row">
                                <span class="detail-label">"Input Size:"</span>
                                <span class="detail-value">"640x640"</span>
                            </div>
                            <div class="detail-row">
                                <span class="detail-label">"Classes:"</span>
                                <span class="detail-value">"80 (COCO)"</span>
                            </div>
                            <div class="detail-row">
                                <span class="detail-label">"Backend:"</span>
                                <span class="detail-value">"tract_onnx"</span>
                            </div>
                        </div>
                    </div>
                    
                    <div class="performance-card">
                        <h3>"Model Performance"</h3>
                        <div class="perf-stats">
                            <div class="perf-item">
                                <span class="perf-label">"Avg Inference"</span>
                                <span class="perf-value">"15.3ms"</span>
                            </div>
                            <div class="perf-item">
                                <span class="perf-label">"Memory Usage"</span>
                                <span class="perf-value">"127MB"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
