# Architecture Documentation

## Overview

This document describes the architecture of the Person Detection System, a real-time video surveillance application built in Rust for Fedora Linux.

## System Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Application Layer                               │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                            App (Orchestrator)                         │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │                  │                  │
                    ▼                  ▼                  ▼
┌─────────────────────┐ ┌──────────────────┐ ┌──────────────────────┐
│   Camera Capture    │ │    Inference     │ │  Detection State     │
│      Task           │ │      Task        │ │     Machine          │
│  ┌───────────────┐  │ │  ┌─────────────┐ │ │  ┌────────────────┐  │
│  │ OpenCV::Video │  │ │  │   tract     │ │ │  │   IDLE         │  │
│  │   Capture     │──┼─┼─▶│   onnx      │─┼─┼─▶│   PERSON_DETECTED │ │
│  └───────────────┘  │ │ │  │   YOLO      │ │ │  │   RECORDING    │  │
│         │           │ │ │  └─────────────┘ │ │  │   GRACE_PERIOD │  │
│         ▼           │ │ │        │         │ │  └────────────────┘  │
│  Bounded Channel    │ │ │        ▼         │ │         │            │
│  (tokio::mpsc)      │ │ │ Bounded Channel  │ │         ▼            │
└─────────────────────┘ │ │ (tokio::mpsc)    │ │  Bounded Channel     │
                        │ └──────────────────┘ │  (tokio::mpsc)       │
                        │                      │                      │
                        │                      │                      ▼
                        │                      │         ┌──────────────────────┐
                        │                      │         │   Recording Task   │
                        │                      │         │  ┌───────────────┐  │
                        │                      │         │  │ VideoWriter   │  │
                        │                      │         │  │ (H.264/MJPEG) │  │
                        │                      │         │  └───────────────┘  │
                        │                      │         └──────────────────────┘
                        │                      │
                        │                      ▼
                        │           ┌──────────────────────┐
                        │           │   Metrics Collector│
                        │           │  (Atomic counters)   │
                        │           └──────────────────────┘
                        ▼
           ┌──────────────────────┐
           │   Configuration      │
           │  (TOML, serde)       │
           └──────────────────────┘
```

## Concurrency Model

### Task Topology

```
┌────────────────────────────────────────────────────────────────┐
│                        Tokio Runtime                            │
│                     (multi-threaded)                            │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │   Camera     │    │  Inference   │    │  Detection   │       │
│  │   Task       │───▶│   Task       │───▶│   Task       │       │
│  │              │    │              │    │              │       │
│  │ - Blocking   │    │ - Preprocess │    │ - State      │       │
│  │   I/O in     │    │   (rayon)    │    │   machine    │       │
│  │   spawn_     │    │ - tract_onnx │    │ - Command    │       │
│  │   blocking   │    │   inference  │    │   dispatch   │       │
│  └──────────────┘    │ - Postprocess│    └──────┬───────┘       │
│                      │   (rayon)    │           │                │
│                      └──────────────┘           ▼                │
│                                        ┌──────────────┐          │
│                                        │   Recording  │          │
│                                        │   Task       │          │
│                                        │              │          │
│                                        │ - Video I/O  │          │
│                                        │   in spawn_  │          │
│                                        │   blocking   │          │
│                                        └──────────────┘          │
│                                                                 │
└────────────────────────────────────────────────────────────────┘

        Bounded Channels (backpressure with stale frame drop)

        Camera ──[Frame]────▶ Inference
        Inference ──[Result]─▶ Detection
        Detection ──[Command]▶ Recording
```

## Module Responsibilities

### `camera.rs`
- **Purpose**: Continuous USB webcam capture
- **Concurrency**: Runs in dedicated tokio task, uses `spawn_blocking` for OpenCV I/O
- **Key Behavior**: Uses `try_send` to drop frames when inference lags (never blocks)
- **Output**: `Frame` structs via bounded channel

### `model.rs`
- **Purpose**: YOLO ONNX model loading and inference
- **Library**: tract_onnx only (no ort, TensorRT, or Python)
- **Performance**: 
  - Preprocessing with rayon parallelism
  - Optimized model loaded once at startup
  - Tensor reuse
- **Output**: Vector of `Detection` structs

### `detection.rs`
- **Purpose**: State machine for recording lifecycle
- **States**: IDLE → PERSON_DETECTED → RECORDING → GRACE_PERIOD → IDLE
- **Key Feature**: 10-second grace period (configurable) after person leaves
- **Output**: Recording commands via bounded channel

### `recording.rs`
- **Purpose**: Video file encoding and writing
- **Format**: H.264 with MJPEG fallback
- **Concurrency**: Blocking I/O in `spawn_blocking`
- **Key Feature**: Graceful file finalization on shutdown

### `app.rs`
- **Purpose**: Main orchestrator, task spawning, shutdown coordination
- **Signal Handling**: SIGINT/SIGTERM with graceful cleanup
- **Metrics**: Periodic performance reporting

## Data Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Capture    │     │  Preprocess │     │   Infer     │     │   Post      │
│  (OpenCV)   │────▶│  (rayon)    │────▶│  (tract)    │────▶│  (rayon)    │
│             │     │             │     │             │     │             │
│ Raw BGR     │     │ Resize +    │     │ YOLO ONNX   │     │ NMS +       │
│ Frame       │     │ Normalize   │     │ Forward     │     │ Filter      │
└─────────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘
                                                                   │
                                                                   ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Finalize  │     │  Recording  │     │   State     │     │  Detection  │
│   File      │◄────│  (OpenCV)   │◄────│  Machine    │◄────│  Filter     │
│             │     │             │     │             │     │             │
│ H.264/MP4   │     │ Write Frame │     │ Logic +     │     │ Class 0     │
│ Output      │     │ to Disk     │     │ Grace Period│     │ Only        │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
```

## Memory Management

### Buffer Strategy
- **Frame data**: `Arc<Mat>` for cheap cloning between tasks
- **Tensors**: Reused inference tensors allocated once
- **Channels**: Bounded to prevent unbounded memory growth

### Preprocessing Optimization
```rust
// Parallel row processing with rayon
rows.into_par_iter().for_each(|(row_idx, row)| {
    // Process each row independently
    // No allocation per row
});
```

## Error Handling

### Strategy
- **Camera errors**: Log and retry, never crash
- **Inference errors**: Skip frame, continue capture
- **Recording errors**: Stop recording, wait for next trigger
- **Shutdown**: Always finalize files, never corrupt output

### Error Types
```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    // ...
}
```

## Performance Characteristics

### Throughput
- **Camera**: 30 FPS (USB bandwidth limited)
- **Inference**: 25-60 FPS depending on hardware
- **Recording**: 30 FPS (disk I/O limited)

### Latency
- **Capture-to-inference**: 1-2 frames (bounded channel)
- **Inference**: 10-30ms (YOLO12n on modern CPU)
- **NMS**: <1ms (parallel processing)

### Scalability
- **CPU-bound**: Preprocessing and NMS scale with rayon thread pool
- **I/O-bound**: Camera and recording use blocking tasks
- **Memory**: O(1) per channel buffer, no unbounded growth

## Shutdown Sequence

```
1. SIGINT/SIGTERM received
   │
   ▼
2. Set shutdown flag (broadcast)
   │
   ├──▶ Camera Task ────────┐
   │   - Stop capturing      │
   │   - Drop channel        │
   │                         │
   ├──▶ Inference Task ──────┤
   │   - Complete current    │
   │   - Drain frame queue   │
   │                         │
   ├──▶ Detection Task ──────┤
   │   - Send Stop command   │
   │   - Complete processing │
   │                         │
   └──▶ Recording Task ──────┤
       - Finalize video file │
       - Flush to disk       │
                             │
                             ▼
                         3. Join all tasks
                             │
                             ▼
                         4. Final metrics
                             │
                             ▼
                         5. Exit cleanly
```

## Future Extensibility

### Planned Integration Points

```
┌────────────────────────────────────────────────────────────────┐
│                    Surveillance System Bus                      │
│                        (Future Integration)                     │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Person     │  │   Motion     │  │   Face       │         │
│  │   Detect     │  │   Detect     │  │   Recognize  │         │
│  │   (This      │  │   (Future)   │  │   (Future)   │         │
│  │   Module)    │  │              │  │              │         │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘         │
│         │                 │                 │                  │
│         └─────────────────┴─────────────────┘                  │
│                           │                                    │
│                           ▼                                    │
│                   ┌──────────────┐                             │
│                   │   Event      │                             │
│                   │   Aggregator │                             │
│                   │   (Kafka/    │                             │
│                   │    MQTT)     │                             │
│                   └──────┬───────┘                             │
│                          │                                     │
│           ┌──────────────┼──────────────┐                     │
│           ▼              ▼              ▼                     │
│     ┌──────────┐   ┌──────────┐   ┌──────────┐              │
│     │  Alert   │   │  Storage │   │  UI      │              │
│     │  System  │   │  System  │   │  Dashboard│              │
│     └──────────┘   └──────────┘   └──────────┘              │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

## Testing Strategy

### Unit Tests (src/tests.rs)
- IoU calculation accuracy
- NMS algorithm correctness
- State machine transitions
- Configuration validation

### Integration Tests
- Model loading and inference (requires model file)
- End-to-end pipeline with sample frames
- Graceful shutdown verification

### Benchmarks
- Inference latency (criterion)
- Preprocessing throughput
- NMS performance with varying detection counts

## Security Considerations

- **Local processing**: No cloud/external service dependencies
- **File permissions**: Recordings saved with restrictive permissions
- **No network**: Single-node deployment by design

## Monitoring and Observability

### Metrics Collected
- Capture FPS
- Inference FPS
- Average inference latency (ms)
- Average NMS latency (ms)
- Total frames processed
- State machine transitions

### Logging Levels
- **ERROR**: Failures that affect functionality
- **WARN**: Recoverable issues (frame drops, codec fallback)
- **INFO**: Major events (recording start/stop, person detection)
- **DEBUG**: Detailed performance data
- **TRACE**: Per-frame processing details
