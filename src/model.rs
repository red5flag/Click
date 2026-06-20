use crate::types::Detection;
use anyhow::{Context, Result};
use opencv::prelude::*;
use rayon::prelude::*;
use std::path::Path;
use std::time::Instant;
use tract_core::prelude::*;
use tract_onnx::prelude::*;
use tract_core::framework::Framework;

/// YOLO ONNX model handler
pub struct YoloModel {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    input_shape: Vec<usize>,
    confidence_threshold: f32,
    iou_threshold: f32,
    target_class: usize,
}

impl YoloModel {
    /// Load and initialize the YOLO model
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        confidence_threshold: f32,
        iou_threshold: f32,
    ) -> Result<Self> {
        let model_path = model_path.as_ref();
        tracing::info!("Loading YOLO model from: {}", model_path.display());

        // Load ONNX model, stripping value_info to avoid unsupported floor() in TDim expressions
        let onnx_parser = tract_onnx::onnx().with_ignore_output_shapes(true);
        let mut proto = onnx_parser
            .proto_model_for_path(model_path)
            .with_context(|| format!("Failed to load ONNX model from {}", model_path.display()))?;

        // tract 0.21 can't parse floor() in dimension expressions; strip value_info
        if let Some(graph) = &mut proto.graph {
            graph.value_info.clear();
        }

        let dir = model_path.parent().and_then(|p| p.to_str());
        let parse_result = onnx_parser
            .parse(&proto, dir)
            .with_context(|| format!("Failed to parse ONNX model from {}", model_path.display()))?;
        if !parse_result.unresolved_inputs.is_empty() {
            return Err(anyhow::anyhow!(
                "Could not resolve inputs at top-level: {:?}",
                parse_result.unresolved_inputs
            ));
        }

        // Provide concrete input shape to avoid symbolic dimension parsing failures
        let typed_model = parse_result
            .model
            .with_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 3, 640, 640)))
            .with_context(|| "Failed to set input fact")?
            .into_optimized()
            .with_context(|| "Failed to optimize model")?;

        // Get input shape from typed model before consuming it
        let input_fact = typed_model
            .input_fact(0)
            .map_err(|e| anyhow::anyhow!("Failed to get input fact: {}", e))?;

        let input_shape: Vec<usize> = input_fact
            .shape
            .iter()
            .map(|dim| {
                dim.to_i64()
                    .map(|v| v as usize)
                    .unwrap_or(640)
            })
            .collect();

        let model = typed_model
            .into_runnable()
            .with_context(|| "Failed to create runnable model")?;

        tracing::info!("Model input shape: {:?}", input_shape);
        tracing::info!("Model loaded successfully");

        Ok(Self {
            model,
            input_shape,
            confidence_threshold,
            iou_threshold,
            target_class: 0, // COCO class 0 = person
        })
    }

    /// Get expected input dimensions
    pub fn input_size(&self) -> (usize, usize) {
        if self.input_shape.len() >= 4 {
            (self.input_shape[2], self.input_shape[3])
        } else {
            (640, 640) // Default size
        }
    }

    /// Preprocess frame for inference (parallel with rayon)
    pub fn preprocess(&self, frame: &opencv::core::Mat) -> Result<Tensor> {
        let (target_h, target_w) = self.input_size();
        let start = Instant::now();

        // Resize frame to model input size
        let mut resized = opencv::core::Mat::default();
        opencv::imgproc::resize(
            frame,
            &mut resized,
            opencv::core::Size::new(target_w as i32, target_h as i32),
            0.0,
            0.0,
            opencv::imgproc::INTER_LINEAR,
        )?;

        // Normalize BGR to [0, 1] and convert to tensor format (HWC RGB)
        let mut tensor_data = vec![0.0f32; target_h * target_w * 3];
        let bgr_data = resized.data_bytes().unwrap_or(&[]);

        // Use rayon for parallel pixel processing without locking
        tensor_data
            .par_chunks_mut(target_w * 3)
            .enumerate()
            .for_each(|(row_idx, chunk)| {
                let row_offset = row_idx * target_w * 3;
                for col in 0..target_w {
                    let src_idx = row_offset + col * 3;
                    let dst_idx = col * 3;
                    if src_idx + 2 < bgr_data.len() {
                        // Convert BGR to RGB and normalize
                        chunk[dst_idx] = bgr_data[src_idx + 2] as f32 / 255.0;
                        chunk[dst_idx + 1] = bgr_data[src_idx + 1] as f32 / 255.0;
                        chunk[dst_idx + 2] = bgr_data[src_idx] as f32 / 255.0;
                    }
                }
            });

        // Create tensor with shape [1, 3, H, W] (NCHW format)
        let tensor = Tensor::from_shape(
            &[1, 3, target_h, target_w],
            &tensor_data,
        )?;

        let elapsed = start.elapsed();
        tracing::debug!("Preprocessing took: {:?}", elapsed);

        Ok(tensor)
    }

    /// Run inference on preprocessed tensor
    pub fn infer(&self, input: Tensor) -> Result<Vec<Detection>> {
        let inference_start = Instant::now();

        // Run inference
        let result = self
            .model
            .run(tvec!(input.into()))
            .map_err(|e| anyhow::anyhow!("Inference failed: {}", e))?;

        let inference_elapsed = inference_start.elapsed();
        
        // Process outputs
        let nms_start = Instant::now();
        let detections = self.process_outputs(&result)?;
        let nms_elapsed = nms_start.elapsed();

        tracing::debug!(
            "Inference: {:?}, NMS: {:?}, Detections: {}",
            inference_elapsed,
            nms_elapsed,
            detections.len()
        );

        Ok(detections)
    }

    /// Process model outputs and apply NMS
    fn process_outputs(&self, outputs: &[TValue]) -> Result<Vec<Detection>> {
        if outputs.is_empty() {
            return Ok(Vec::new());
        }

        // YOLOv8/YOLO12 output format: [batch, num_predictions, 85]
        // where 85 = [x, y, w, h, confidence, 80 class scores]
        let output = &outputs[0];
        let output_view = output.as_slice::<f32>().map_err(|e| anyhow::anyhow!("Failed to slice output: {}", e))?;
        let shape = output.shape();

        if shape.len() != 3 {
            return Err(anyhow::anyhow!(
                "Unexpected output shape: {:?}",
                shape
            ));
        }

        let num_predictions = shape[1];
        let num_classes = shape[2] - 5; // 85 - 5 = 80 for COCO
        let (img_h, img_w) = self.input_size();

        // Collect all detections (parallel processing)
        let mut detections: Vec<Detection> = (0..num_predictions)
            .into_par_iter()
            .filter_map(|i| {
                let offset = i * shape[2];
                
                // Bounding box coordinates (normalized)
                let x = output_view[offset];
                let y = output_view[offset + 1];
                let w = output_view[offset + 2];
                let h = output_view[offset + 3];
                
                // Objectness confidence (YOLOv8 uses class-specific confidence directly)
                let confidence = output_view[offset + 4];
                
                if confidence < self.confidence_threshold {
                    return None;
                }

                // Find class with highest score
                let mut max_class = 0;
                let mut max_score = 0.0f32;
                
                for c in 0..num_classes {
                    let score = output_view[offset + 5 + c];
                    if score > max_score {
                        max_score = score;
                        max_class = c;
                    }
                }

                // Only keep person class (class 0)
                if max_class != self.target_class {
                    return None;
                }

                // Combined confidence
                let combined_conf = confidence * max_score;
                if combined_conf < self.confidence_threshold {
                    return None;
                }

                // Normalize coordinates
                let cx = (x / img_w as f32).clamp(0.0, 1.0);
                let cy = (y / img_h as f32).clamp(0.0, 1.0);
                let nw = (w / img_w as f32).clamp(0.0, 1.0);
                let nh = (h / img_h as f32).clamp(0.0, 1.0);

                Some(Detection::new(cx, cy, nw, nh, combined_conf, max_class))
            })
            .collect();

        // Sort by confidence descending
        detections.par_sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap()
        });

        // Apply NMS
        let filtered = self.apply_nms(detections);

        Ok(filtered)
    }

    /// Apply Non-Maximum Suppression using rayon
    fn apply_nms(&self, detections: Vec<Detection>) -> Vec<Detection> {
        if detections.is_empty() {
            return detections;
        }

        let mut keep = vec![true; detections.len()];
        
        for i in 0..detections.len() {
            if !keep[i] {
                continue;
            }
            
            for j in (i + 1)..detections.len() {
                if !keep[j] {
                    continue;
                }
                
                let iou = detections[i].iou(&detections[j]);
                if iou > self.iou_threshold {
                    keep[j] = false;
                }
            }
        }

        detections
            .into_iter()
            .enumerate()
            .filter_map(|(i, d)| if keep[i] { Some(d) } else { None })
            .collect()
    }

    /// Get model information for logging
    pub fn model_info(&self) -> ModelInfo {
        ModelInfo {
            input_shape: self.input_shape.clone(),
            confidence_threshold: self.confidence_threshold,
            iou_threshold: self.iou_threshold,
            target_class: self.target_class,
        }
    }
}

/// Model information for diagnostics
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub input_shape: Vec<usize>,
    pub confidence_threshold: f32,
    pub iou_threshold: f32,
    pub target_class: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_iou() {
        let d1 = Detection::new(0.5, 0.5, 0.2, 0.2, 0.9, 0);
        let d2 = Detection::new(0.5, 0.5, 0.2, 0.2, 0.8, 0);
        let iou = d1.iou(&d2);
        assert!((iou - 1.0).abs() < 0.001);

        let d3 = Detection::new(0.8, 0.8, 0.2, 0.2, 0.9, 0);
        let iou2 = d1.iou(&d3);
        assert!(iou2 < 0.1);
    }
}
