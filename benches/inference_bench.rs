use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark placeholder for inference performance testing
/// Requires model file to run actual benchmarks
fn nms_benchmark(c: &mut Criterion) {
    // Mock detection data for NMS benchmarking
    let detections: Vec<(f32, f32, f32, f32, f32)> = (0..100)
        .map(|i| {
            let x = (i as f32 % 10.0) * 0.1 + 0.05;
            let y = (i as f32 / 10.0) * 0.1 + 0.05;
            (x, y, 0.1, 0.1, 0.5 + (i as f32 / 200.0))
        })
        .collect();

    c.bench_function("nms_100_detections", |b| {
        b.iter(|| {
            // Simulate NMS logic
            let mut kept = vec![true; detections.len()];
            for i in 0..detections.len() {
                if !kept[i] {
                    continue;
                }
                let (x1, y1, w1, h1, _) = detections[i];
                for j in (i + 1)..detections.len() {
                    if !kept[j] {
                        continue;
                    }
                    let (x2, y2, w2, h2, _) = detections[j];
                    // Simple IoU calculation
                    let intersection = (w1.min(w2) * h1.min(h2)).max(0.0);
                    let union = w1 * h1 + w2 * h2 - intersection;
                    let iou = if union > 0.0 {
                        intersection / union
                    } else {
                        0.0
                    };
                    if iou > 0.45 {
                        kept[j] = false;
                    }
                }
            }
            black_box(kept);
        });
    });
}

fn iou_benchmark(c: &mut Criterion) {
    c.bench_function("iou_calculation", |b| {
        let box1 = (0.5_f32, 0.5_f32, 0.2_f32, 0.4_f32);
        let box2 = (0.6_f32, 0.6_f32, 0.3_f32, 0.3_f32);

        b.iter(|| {
            let (x1, y1, w1, h1) = box1;
            let (x2, y2, w2, h2) = box2;

            let x_left = (x1 - w1 / 2.0).max(x2 - w2 / 2.0);
            let y_top = (y1 - h1 / 2.0).max(y2 - h2 / 2.0);
            let x_right = (x1 + w1 / 2.0).min(x2 + w2 / 2.0);
            let y_bottom = (y1 + h1 / 2.0).min(y2 + h2 / 2.0);

            let intersection_w = (x_right - x_left).max(0.0);
            let intersection_h = (y_bottom - y_top).max(0.0);
            let intersection = intersection_w * intersection_h;

            let area1 = w1 * h1;
            let area2 = w2 * h2;
            let union = area1 + area2 - intersection;

            let iou = if union > 0.0 {
                intersection / union
            } else {
                0.0
            };

            black_box(iou);
        });
    });
}

criterion_group!(benches, nms_benchmark, iou_benchmark);
criterion_main!(benches);
