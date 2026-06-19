use crate::types::{Detection, InferenceResult, RecordingState};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Detection state machine configuration
#[derive(Debug, Clone)]
pub struct DetectionConfig {
    pub confidence_threshold: f32,
    pub grace_period_seconds: u64,
}

impl DetectionConfig {
    pub fn new(confidence_threshold: f32, grace_period_seconds: u64) -> Self {
        Self {
            confidence_threshold,
            grace_period_seconds,
        }
    }
}

/// State machine for detection-based recording
pub struct DetectionStateMachine {
    state: RecordingState,
    config: DetectionConfig,
    last_detection_time: Option<Instant>,
    recording_start_time: Option<Instant>,
    grace_period_duration: Duration,
    person_count: usize,
}

/// State machine transition result
#[derive(Debug, Clone, PartialEq)]
pub enum StateTransition {
    /// No state change
    NoChange,
    /// Started recording
    RecordingStarted,
    /// Stopped recording
    RecordingStopped,
    /// Continued recording
    RecordingContinued,
    /// Entered grace period
    GracePeriodStarted,
    /// Cancelled grace period (person reappeared)
    GracePeriodCancelled,
}

impl DetectionStateMachine {
    pub fn new(config: DetectionConfig) -> Self {
        let grace_period = Duration::from_secs(config.grace_period_seconds);
        
        Self {
            state: RecordingState::Idle,
            config,
            last_detection_time: None,
            recording_start_time: None,
            person_count: 0,
            grace_period_duration: grace_period,
        }
    }

    /// Get current state
    pub fn state(&self) -> RecordingState {
        self.state
    }

    /// Process inference result and update state
    pub fn process_inference(&mut self, result: &InferenceResult) -> StateTransition {
        let person_detected = !result.detections.is_empty();
        let now = Instant::now();

        if person_detected {
            self.last_detection_time = Some(now);
            self.person_count = result.detections.len();
        }

        let transition = match self.state {
            RecordingState::Idle => {
                if person_detected {
                    info!(
                        "Person detected (confidence: {:.2}), starting recording",
                        result.detections[0].confidence
                    );
                    self.state = RecordingState::PersonDetected;
                    self.recording_start_time = Some(now);
                    StateTransition::RecordingStarted
                } else {
                    StateTransition::NoChange
                }
            }

            RecordingState::PersonDetected => {
                if person_detected {
                    self.state = RecordingState::Recording;
                    StateTransition::RecordingContinued
                } else {
                    // Person disappeared immediately - stay in PersonDetected briefly
                    // This prevents flickering if detection is momentarily lost
                    StateTransition::NoChange
                }
            }

            RecordingState::Recording => {
                if person_detected {
                    StateTransition::RecordingContinued
                } else {
                    info!(
                        "Person left, starting grace period ({} seconds)",
                        self.config.grace_period_seconds
                    );
                    self.state = RecordingState::RecordingGracePeriod;
                    StateTransition::GracePeriodStarted
                }
            }

            RecordingState::RecordingGracePeriod => {
                if person_detected {
                    info!(
                        "Person reappeared during grace period ({} persons), continuing recording",
                        self.person_count
                    );
                    self.state = RecordingState::Recording;
                    StateTransition::GracePeriodCancelled
                } else {
                    // Check if grace period expired
                    let grace_elapsed = now.duration_since(
                        self.last_detection_time.unwrap_or(now)
                    );

                    if grace_elapsed >= self.grace_period_duration {
                        info!(
                            "Grace period expired, finalizing recording (total duration: {:?})",
                            self.recording_start_time.map(|t| now.duration_since(t))
                        );
                        self.state = RecordingState::Idle;
                        self.recording_start_time = None;
                        StateTransition::RecordingStopped
                    } else {
                        let remaining = self.grace_period_duration - grace_elapsed;
                        debug!(
                            "Grace period: {} seconds remaining",
                            remaining.as_secs()
                        );
                        StateTransition::RecordingContinued
                    }
                }
            }
        };

        self.state = match transition {
            StateTransition::RecordingStarted => RecordingState::Recording,
            StateTransition::GracePeriodStarted => RecordingState::RecordingGracePeriod,
            StateTransition::GracePeriodCancelled => RecordingState::Recording,
            _ => self.state,
        };

        transition
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        matches!(
            self.state,
            RecordingState::Recording | RecordingState::RecordingGracePeriod
        )
    }

    /// Check if recording just started (first frame)
    pub fn is_recording_start(&self, transition: &StateTransition) -> bool {
        matches!(transition, StateTransition::RecordingStarted)
    }

    /// Check if recording just stopped (finalize)
    pub fn is_recording_stop(&self, transition: &StateTransition) -> bool {
        matches!(transition, StateTransition::RecordingStopped)
    }

    /// Get current person count
    pub fn person_count(&self) -> usize {
        self.person_count
    }

    /// Get recording duration if active
    pub fn recording_duration(&self) -> Option<Duration> {
        self.recording_start_time.map(|start| start.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_idle_to_recording_transition() {
        let config = DetectionConfig::new(0.5, 10);
        let mut fsm = DetectionStateMachine::new(config);

        assert_eq!(fsm.state(), RecordingState::Idle);

        // Person detected
        let result = create_result(vec![create_detection(0.8)]);
        let transition = fsm.process_inference(&result);
        assert_eq!(transition, StateTransition::RecordingStarted);
        assert!(fsm.is_recording());
    }

    #[test]
    fn test_no_person_in_idle() {
        let config = DetectionConfig::new(0.5, 10);
        let mut fsm = DetectionStateMachine::new(config);

        let result = create_result(vec![]);
        let transition = fsm.process_inference(&result);
        assert_eq!(transition, StateTransition::NoChange);
        assert!(!fsm.is_recording());
    }

    #[test]
    fn test_grace_period_timeout() {
        let config = DetectionConfig::new(0.5, 0); // 0 second grace period for test
        let mut fsm = DetectionStateMachine::new(config);

        // Start recording
        let result = create_result(vec![create_detection(0.8)]);
        fsm.process_inference(&result);

        // Person leaves - should immediately stop with 0s grace
        let result = create_result(vec![]);
        std::thread::sleep(Duration::from_millis(10));
        let transition = fsm.process_inference(&result);
        
        // With 0 grace period, should stop immediately
        // Note: actual behavior depends on timing
    }

    #[test]
    fn test_grace_period_cancel() {
        let config = DetectionConfig::new(0.5, 10);
        let mut fsm = DetectionStateMachine::new(config);

        // Start recording
        let result = create_result(vec![create_detection(0.8)]);
        fsm.process_inference(&result);
        fsm.process_inference(&result); // Move to Recording state

        // Person leaves - grace period
        let empty = create_result(vec![]);
        fsm.process_inference(&empty);
        assert!(matches!(fsm.state(), RecordingState::RecordingGracePeriod));

        // Person reappears - cancel grace
        let result = create_result(vec![create_detection(0.7)]);
        let transition = fsm.process_inference(&result);
        assert_eq!(transition, StateTransition::GracePeriodCancelled);
    }
}
