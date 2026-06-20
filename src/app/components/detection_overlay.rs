use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos::control_flow::For;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct DetectionBox {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub confidence: f64,
    pub class_name: String,
}

#[component]
pub fn DetectionOverlay(detections: ReadSignal<Vec<DetectionBox>>) -> impl IntoView {
    view! {
        <div class="detection-overlay-container">
            <For
                each=move || detections.get()
                key=|d| d.id.clone()
                children=move |detection| {
                    view! {
                        <DetectionBoxComponent detection=detection/>
                    }
                }
            />
        </div>
    }
}

#[component]
fn DetectionBoxComponent(detection: DetectionBox) -> impl IntoView {
    let box_style = move || {
        format!(
            "left: {}%; top: {}%; width: {}%; height: {}%;",
            detection.x * 100.0,
            detection.y * 100.0,
            detection.width * 100.0,
            detection.height * 100.0
        )
    };
    
    let confidence_color = move || {
        if detection.confidence > 0.8 {
            "#22c55e"
        } else if detection.confidence > 0.6 {
            "#f59e0b"
        } else {
            "#ef4444"
        }
    };
    
    view! {
        <div class="detection-bounding-box" style=box_style>
            <div class="detection-border" style=move || format!("border-color: {}", confidence_color())/>
            <div class="detection-label" style=move || format!("background-color: {}", confidence_color())>
                <span>{detection.class_name.clone()}</span>
                <span class="confidence">{format!(" {:.0}%", detection.confidence * 100.0)}</span>
            </div>
        </div>
    }
}
