# Person Detection System

A production-quality Rust application for real-time person-only detection using YOLO12n ONNX model with the tract_onnx inference engine.

## Features

- **Real-time detection**: Captures from USB webcam, runs YOLO inference, detects persons only (COCO class 0)
- **Smart recording**: Automatically starts recording when a person is detected, continues for a configurable grace period after they leave
- **High performance**: Optimized tract_onnx inference with tensor reuse, rayon parallelism for preprocessing and postprocessing
- **Async architecture**: tokio-based concurrent tasks for capture, inference, state machine, and recording
- **Metrics**: Capture FPS, inference FPS, latency measurements with periodic logging
- **Production ready**: Clean shutdown on SIGINT/SIGTERM, proper file handling, modular architecture

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Camera    │────▶│  Inference  │────▶│   Detection │────▶│  Recording  │
│   Capture   │     │   Engine    │     │   State     │     │    Task     │
│   (Tokio)   │     │  (tract)    │     │   Machine   │     │  (OpenCV)   │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
       │                   │                     │                    │
       ▼                   ▼                     ▼                    ▼
   Bounded            Bounded              Bounded              Bounded
   Channel            Channel              Channel              Channel
   (drop stale)       (drop stale)         (drop stale)         (drop stale)
```

### Task Design

1. **Camera Task**: Continuously captures frames, drops stale frames if inference lags
2. **Inference Task**: Preprocesses with rayon parallelism, runs tract_onnx inference
3. **Detection Task**: State machine managing recording lifecycle with grace period
4. **Recording Task**: H.264 video encoding with fallback to MJPEG

## Requirements

- **OS**: Fedora Linux 44 (or compatible)
- **Rust**: 1.75+ (stable channel)
- **Libraries**: OpenCV 4.x development libraries
- **Hardware**: USB webcam, CPU with AVX2 support recommended

## Build Instructions (Fedora 44)

### 1. Install System Dependencies

```bash
# Install OpenCV development libraries
sudo dnf install opencv opencv-devel

# Install additional dependencies
sudo dnf install clang llvm-devel

# Install pkg-config
sudo dnf install pkgconfig
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Clone and Build

```bash
cd /home/red/PersonDetect
cargo build --release
```

### 4. Download YOLO Model

```bash
mkdir -p models
# Download yolo12n.onnx from your preferred source
# Example: wget https://example.com/yolo12n.onnx -O models/yolo12n.onnx
```

### 5. Run

```bash
# With default configuration
cargo run --release

# With custom configuration
PERSON_DETECT_CONFIG=./my-config.toml cargo run --release
```

## Configuration

Create a `config.toml` file in the project root or at `~/.config/person-detect/config.toml`:

```toml
# Camera settings
camera_index = 0

# Detection thresholds
confidence_threshold = 0.5
iou_threshold = 0.45

# Recording settings
grace_period_seconds = 10
recordings_directory = "./recordings"
recording_fps = 30.0
video_codec = "h264"

# Performance settings
max_channel_depth = 3
metrics_interval_seconds = 30

# Model path
model_path = "./models/yolo12n.onnx"
```

## Project Structure

```
.
├── Cargo.toml              # Rust dependencies
├── config.toml            # Example configuration
├── README.md              # This file
├── ARCHITECTURE.md        # Detailed architecture docs
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Module exports
│   ├── app.rs            # Main orchestrator
│   ├── camera.rs         # USB camera capture
│   ├── config.rs         # Configuration management
│   ├── detection.rs      # State machine logic
│   ├── metrics.rs        # Performance metrics
│   ├── model.rs          # YOLO ONNX inference
│   ├── recording.rs      # Video recording
│   ├── types.rs          # Shared types
│   └── tests.rs          # Unit tests
├── models/               # ONNX model files
│   └── yolo12n.onnx     # YOLO12n model
└── recordings/          # Output directory
```

## Usage

### Running

```bash
# Basic run
cargo run --release

# With environment logging
RUST_LOG=info cargo run --release

# With debug logging
RUST_LOG=debug cargo run --release
```

### Testing

```bash
# Run unit tests
cargo test

# Run with output
cargo test -- --nocapture
```

### Output

Recordings are saved to `./recordings/` with format: `YYYY-MM-DD_HH-MM-SS.mp4`

## Logging

The application uses `tracing` for structured logging:

```
2024-01-15T08:30:00.123Z INFO person_detect: Person Detection System Starting
2024-01-15T08:30:00.234Z INFO person_detect: Camera opened: 1920x1080 @ 30.0 FPS
2024-01-15T08:30:01.456Z INFO person_detect: Person detected (confidence: 0.87), starting recording
2024-01-15T08:30:01.567Z INFO person_detect: Recording started: ./recordings/2024-01-15_08-30-01.mp4
2024-01-15T08:30:30.000Z INFO person_detect: Performance metrics: capture_fps=30.1, inference_fps=28.5, avg_inference_ms=15.2, avg_nms_ms=0.8
```

## Performance

Typical performance on modern hardware:
- Capture: 30 FPS (camera limited)
- Inference: 25-30 FPS with YOLO12n
- Latency: 10-20ms inference, <1ms NMS

## Graceful Shutdown

The application handles SIGINT (Ctrl+C) and SIGTERM for clean shutdown:
- Stops camera capture immediately
- Completes pending inference
- Finalizes active recordings
- Flushes all files

## License

MIT License

## Contributing

This is a production reference implementation. For production deployment, consider:
- Adding authentication/authorization
- Implementing remote configuration updates
- Adding health check endpoints
- Setting up log aggregation
- Implementing alerting for anomalies
# Click
