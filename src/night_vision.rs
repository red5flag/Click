use opencv::prelude::*;
use tracing::{debug, info};

/// Low-light mode controller for automatic night vision.
///
/// Monitors average frame brightness and adjusts camera settings when it
/// falls below a threshold. The camera must support the V4L/DSHOW brightness,
/// gain and exposure properties for this to have an effect.
#[derive(Debug, Clone)]
pub struct NightVision {
    pub enabled: bool,
    pub threshold: u8,
    /// Number of consecutive frames below/above threshold before switching
    pub frame_history: usize,
    low_light_frames: usize,
    normal_frames: usize,
    active: bool,
}

impl NightVision {
    pub fn new(enabled: bool, threshold: u8, frame_history: usize) -> Self {
        Self {
            enabled,
            threshold,
            frame_history,
            low_light_frames: 0,
            normal_frames: 0,
            active: false,
        }
    }

    /// Compute average brightness of a BGR frame by converting to grayscale.
    pub fn average_brightness(frame: &opencv::core::Mat) -> u8 {
        if frame.empty() {
            return 255;
        }
        let mut gray = opencv::core::Mat::default();
        if opencv::imgproc::cvt_color(
            frame,
            &mut gray,
            opencv::imgproc::COLOR_BGR2GRAY,
            0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
        )
        .is_err()
        {
            return 255;
        }
        let mut mean = opencv::core::Mat::default();
        let mut stddev = opencv::core::Mat::default();
        if opencv::core::mean_std_dev(&gray, &mut mean, &mut stddev, &opencv::core::Mat::default()).is_err() {
            return 255;
        }
        let data = mean.data_typed::<f64>().unwrap_or(&[]);
        data.first().map(|v| *v as u8).unwrap_or(255)
    }

    /// Process a frame and update camera settings if needed.
    /// Returns true when low-light mode is active.
    pub fn process_frame(
        &mut self,
        frame: &opencv::core::Mat,
        camera: &mut opencv::videoio::VideoCapture,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        let brightness = Self::average_brightness(frame);
        debug!("Average frame brightness: {}", brightness);

        if brightness < self.threshold {
            self.low_light_frames += 1;
            self.normal_frames = 0;
        } else {
            self.normal_frames += 1;
            self.low_light_frames = 0;
        }

        if !self.active && self.low_light_frames >= self.frame_history {
            info!("Low light detected (brightness {}), enabling night vision", brightness);
            self.apply_night_mode(camera);
            self.active = true;
        } else if self.active && self.normal_frames >= self.frame_history {
            info!("Light level restored (brightness {}), disabling night vision", brightness);
            self.apply_normal_mode(camera);
            self.active = false;
        }

        self.active
    }

    fn apply_night_mode(&self, camera: &mut opencv::videoio::VideoCapture) {
        // Best-effort adjustments; many cameras ignore some or all of these.
        let _ = camera.set(opencv::videoio::CAP_PROP_BRIGHTNESS, 100.0);
        let _ = camera.set(opencv::videoio::CAP_PROP_GAIN, 100.0);
        // Set exposure to manual/low value. The exact value is camera-specific.
        let _ = camera.set(opencv::videoio::CAP_PROP_EXPOSURE, -6.0);
        let _ = camera.set(opencv::videoio::CAP_PROP_BACKLIGHT, 1.0);
    }

    fn apply_normal_mode(&self, camera: &mut opencv::videoio::VideoCapture) {
        let _ = camera.set(opencv::videoio::CAP_PROP_BRIGHTNESS, 50.0);
        let _ = camera.set(opencv::videoio::CAP_PROP_GAIN, 50.0);
        let _ = camera.set(opencv::videoio::CAP_PROP_EXPOSURE, -4.0);
        let _ = camera.set(opencv::videoio::CAP_PROP_BACKLIGHT, 0.0);
    }
}
