// Unit and integration tests for person-detect

use super::*;
use crate::detection::{DetectionConfig, DetectionStateMachine, StateTransition};
use crate::model::YoloModel;
use crate::types::{Detection, InferenceResult, RecordingState};
use tempfile::TempDir;

// ============================================================================
// NMS Tests
// ============================================================================

#[test]
fn test_iou_identical_boxes() {
    let d1 = Detection::new(0.5, 0.5, 0.2, 0.2, 0.9, 0);
    let d2 = Detection::new(0.5, 0.5, 0.2, 0.2, 0.8, 0);
    let iou = d1.iou(&d2);
    assert!((iou - 1.0).abs() < 0.001, "Expected IoU ~1.0, got {}", iou);
}

#[test]
fn test_iou_non_overlapping() {
    let d1 = Detection::new(0.1, 0.1, 0.1, 0.1, 0.9, 0);
    let d2 = Detection::new(0.9, 0.9, 0.1, 0.1, 0.9, 0);
    let iou = d1.iou(&d2);
    assert!(iou < 0.01, "Expected IoU ~0.0, got {}", iou);
}

#[test]
fn test_iou_partial_overlap() {
    let d1 = Detection::new(0.5, 0.5, 0.4, 0.4, 0.9, 0);
    let d2 = Detection::new(0.6, 0.6, 0.4, 0.4, 0.8, 0);
    let iou = d1.iou(&d2);
    // Overlap area ~0.12, union ~0.44, IoU ~0.27
    assert!(iou > 0.2 && iou < 0.4, "Expected IoU ~0.27, got {}", iou);
}

#[test]
fn test_iou_zero_area() {
    let d1 = Detection::new(0.5, 0.5, 0.0, 0.0, 0.9, 0);
    let d2 = Detection::new(0.5, 0.5, 0.2, 0.2, 0.8, 0);
    let iou = d1.iou(&d2);
    assert_eq!(iou, 0.0, "Expected IoU 0.0 for zero area");
}

// ============================================================================
// Detection Filtering Tests
// ============================================================================

#[test]
fn test_detection_class_filtering() {
    // Only class 0 (person) should be kept
    let person_det = Detection::new(0.5, 0.5, 0.2, 0.4, 0.8, 0);
    let car_det = Detection::new(0.5, 0.5, 0.3, 0.3, 0.9, 2); // Car
    
    assert_eq!(person_det.class_id, 0);
    assert_eq!(car_det.class_id, 2);
    
    // In actual model, we filter by class_id == 0
    assert!(person_det.class_id == 0);
    assert!(car_det.class_id != 0);
}

#[test]
fn test_detection_confidence_threshold() {
    let high_conf = Detection::new(0.5, 0.5, 0.2, 0.4, 0.8, 0);
    let low_conf = Detection::new(0.5, 0.5, 0.2, 0.4, 0.3, 0);
    
    let threshold = 0.5f32;
    assert!(high_conf.confidence >= threshold);
    assert!(low_conf.confidence < threshold);
}

// ============================================================================
// State Machine Tests
// ============================================================================

fn create_detection(confidence: f32) -> Detection {
    Detection::new(0.5, 0.5, 0.2, 0.4, confidence, 0)
}

fn create_result(detections: Vec<Detection>) -> InferenceResult {
    InferenceResult {
        detections,
        inference_latency_ms: 10.0,
        nms_latency_ms: 1.0,
        sequence: 1,
    }
}

#[test]
fn test_state_machine_idle_to_recording() {
    let config = DetectionConfig::new(0.5, 10);
    let mut fsm = DetectionStateMachine::new(config);

    assert_eq!(fsm.state(), RecordingState::Idle);
    assert!(!fsm.is_recording());

    // Person detected - should start recording
    let result = create_result(vec![create_detection(0.8)]);
    let transition = fsm.process_inference(&result);
    
    assert_eq!(transition, StateTransition::RecordingStarted);
    assert!(fsm.is_recording());
    assert_eq!(fsm.person_count(), 1);
}

#[test]
fn test_state_machine_no_person_idle() {
    let config = DetectionConfig::new(0.5, 10);
    let mut fsm = DetectionStateMachine::new(config);

    let result = create_result(vec![]);
    let transition = fsm.process_inference(&result);
    
    assert_eq!(transition, StateTransition::NoChange);
    assert_eq!(fsm.state(), RecordingState::Idle);
    assert!(!fsm.is_recording());
}

#[test]
fn test_state_machine_recording_to_grace() {
    let config = DetectionConfig::new(0.5, 10);
    let mut fsm = DetectionStateMachine::new(config);

    // Start recording
    let result = create_result(vec![create_detection(0.8)]);
    fsm.process_inference(&result);
    fsm.process_inference(&result); // Move to Recording state

    // Person leaves
    let empty = create_result(vec![]);
    let transition = fsm.process_inference(&empty);

    assert_eq!(transition, StateTransition::GracePeriodStarted);
    assert_eq!(fsm.state(), RecordingState::RecordingGracePeriod);
    assert!(fsm.is_recording()); // Still recording during grace
}

#[test]
fn test_state_machine_grace_cancel() {
    let config = DetectionConfig::new(0.5, 10);
    let mut fsm = DetectionStateMachine::new(config);

    // Start and enter recording
    let result = create_result(vec![create_detection(0.8)]);
    fsm.process_inference(&result);
    fsm.process_inference(&result);

    // Enter grace period
    let empty = create_result(vec![]);
    fsm.process_inference(&empty);

    // Person reappears - cancel grace
    let new_result = create_result(vec![create_detection(0.7)]);
    let transition = fsm.process_inference(&new_result);

    assert_eq!(transition, StateTransition::GracePeriodCancelled);
    assert_eq!(fsm.state(), RecordingState::Recording);
    assert!(fsm.is_recording());
}

#[test]
fn test_state_machine_multiple_persons() {
    let config = DetectionConfig::new(0.5, 10);
    let mut fsm = DetectionStateMachine::new(config);

    // Multiple persons detected
    let result = create_result(vec![
        create_detection(0.9),
        create_detection(0.8),
        create_detection(0.7),
    ]);
    
    fsm.process_inference(&result);
    assert_eq!(fsm.person_count(), 3);
}

#[test]
fn test_state_machine_recording_duration() {
    let config = DetectionConfig::new(0.5, 10);
    let mut fsm = DetectionStateMachine::new(config);

    // Not recording - no duration
    assert!(fsm.recording_duration().is_none());

    // Start recording
    let result = create_result(vec![create_detection(0.8)]);
    fsm.process_inference(&result);
    fsm.process_inference(&result);

    // Should have duration
    std::thread::sleep(std::time::Duration::from_millis(10));
    let duration = fsm.recording_duration();
    assert!(duration.is_some());
    assert!(duration.unwrap().as_millis() >= 10);
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_config_default() {
    let config = config::Config::default();
    assert_eq!(config.camera_index, 0);
    assert_eq!(config.confidence_threshold, 0.5);
    assert_eq!(config.iou_threshold, 0.45);
    assert_eq!(config.grace_period_seconds, 10);
    assert_eq!(config.recordings_directory, "./recordings");
    assert_eq!(config.max_channel_depth, 3);
}

#[test]
fn test_config_validation_valid() {
    let config = config::Config::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_invalid_confidence() {
    let mut config = config::Config::default();
    config.confidence_threshold = 1.5;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_invalid_iou() {
    let mut config = config::Config::default();
    config.iou_threshold = -0.1;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    let original = config::Config::default();
    original.save(&config_path).unwrap();

    let loaded = config::Config::from_file(&config_path).unwrap();
    assert_eq!(original.camera_index, loaded.camera_index);
    assert_eq!(original.confidence_threshold, loaded.confidence_threshold);
    assert_eq!(original.iou_threshold, loaded.iou_threshold);
}

// ============================================================================
// Integration Test (requires model file)
// ============================================================================

#[cfg(feature = "integration-tests")]
mod integration {
    use super::*;
    use opencv::prelude::*;
    use opencv::core::Mat;
    use opencv::imgproc;

    #[test]
    fn test_model_load_and_inference() {
        // This test requires the actual model file
        let model_path = "./models/model.onnx";
        if !std::path::Path::new(model_path).exists() {
            println!("Skipping integration test - model not found");
            return;
        }

        let model = YoloModel::new(model_path, 0.5, 0.45)
            .expect("Failed to load model");

        let (h, w) = model.input_size();
        println!("Model input size: {}x{}", h, w);

        // Create test frame
        let mut test_frame = Mat::default();
        imgproc::cvt_color(
            &Mat::new_rows_cols_with_default(h as i32, w as i32, opencv::core::CV_8UC3, opencv::core::Scalar::all(128.0)).unwrap(),
            &mut test_frame,
            imgproc::COLOR_BGR2RGB,
            0,
        ).unwrap();

        // Preprocess
        let tensor = model.preprocess(&test_frame)
            .expect("Preprocessing failed");

        // Infer
        let detections = model.infer(tensor)
            .expect("Inference failed");

        println!("Detections: {:?}", detections);
    }
}
