# Build Instructions for Fedora 44

## Prerequisites

### System Requirements
- Fedora Linux 44 (or compatible RHEL/CentOS 9)
- 4GB RAM minimum (8GB recommended)
- 10GB free disk space
- USB webcam (V4L2 compatible)

### Required Packages

```bash
# Update system
sudo dnf update -y

# Install Rust toolchain dependencies
sudo dnf groupinstall -y "Development Tools" "Development Libraries"

# Install OpenCV (required for camera capture and video encoding)
sudo dnf install -y opencv opencv-devel opencv-contrib

# Install additional system libraries
sudo dnf install -y \
    clang \
    llvm-devel \
    pkgconfig \
    cmake \
    git \
    wget

# Install BLAS/LAPACK for tract_onnx optimization
sudo dnf install -y \
    openblas-devel \
    lapack-devel
```

## Rust Installation

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts to complete installation
# Then reload your shell environment:
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

Expected versions:
- rustc 1.75.0 or later
- cargo 1.75.0 or later

## Project Setup

```bash
# Clone or navigate to the project
cd /home/red/PersonDetect

# The project structure should be:
# .
# ├── Cargo.toml
# ├── config.toml.example
# ├── src/
# │   ├── main.rs
# │   ├── lib.rs
# │   ├── app.rs
# │   ├── camera.rs
# │   ├── config.rs
# │   ├── detection.rs
# │   ├── metrics.rs
# │   ├── model.rs
# │   ├── recording.rs
# │   ├── types.rs
# │   └── tests.rs
# ├── benches/
# ├── models/
# └── recordings/
```

## Model Setup

The application requires a YOLO ONNX model. You can obtain yolo12n.onnx from:

1. **Ultralytics official releases** (convert to ONNX):
   ```bash
   # Requires Python with ultralytics
   pip install ultralytics
   yolo export model=yolo12n.pt format=onnx
   ```

2. **Direct download** (replace with actual URL):
   ```bash
   mkdir -p models
   wget https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo12n.onnx \
        -O models/yolo12n.onnx
   ```

3. **Manual conversion**:
   - Export from PyTorch using `torch.onnx.export`
   - Ensure input shape is [1, 3, 640, 640] or detectable from graph

Place the model at:
```
/home/red/PersonDetect/models/yolo12n.onnx
```

## Build Steps

### Development Build

```bash
cd /home/red/PersonDetect

# Build with debug symbols
cargo build

# The binary will be at:
# ./target/debug/person-detect
```

### Release Build (Production)

```bash
cd /home/red/PersonDetect

# Build optimized release
cargo build --release

# The binary will be at:
# ./target/release/person-detect
```

### Build Features

The `Cargo.toml` includes these optimizations for release:
- `opt-level = 3` - Maximum optimization
- `lto = "thin"` - Link-time optimization
- `codegen-units = 1` - Better optimization at cost of slower compile
- `strip = true` - Remove debug symbols for smaller binary
- `panic = "abort"` - Smaller binary, no unwinding

## Configuration

Copy the example configuration:

```bash
cp config.toml.example config.toml

# Edit as needed
nano config.toml
```

Key settings to verify:
- `camera_index` - Set to your USB camera index (usually 0)
- `model_path` - Path to yolo12n.onnx
- `recordings_directory` - Where videos will be saved

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_state_machine
```

### Integration Tests

```bash
# Integration tests require the model file
# They are disabled by default (feature flag)

# To run with model (if available):
cargo test --features integration-tests
```

### Benchmarks

```bash
# Run performance benchmarks
cargo bench

# Results will be in:
# ./target/criterion/
```

## Running the Application

### Basic Run

```bash
# Using cargo
cargo run --release

# Or using the built binary
./target/release/person-detect
```

### With Logging

```bash
# Info level logging
RUST_LOG=info cargo run --release

# Debug level (verbose)
RUST_LOG=debug cargo run --release

# Trace level (very verbose, per-frame)
RUST_LOG=trace cargo run --release
```

### With Custom Config

```bash
# Via environment variable
PERSON_DETECT_CONFIG=/etc/person-detect/config.toml cargo run --release

# Or place config.toml in working directory
```

## Verification

### Check Camera

```bash
# List video devices
ls -la /dev/video*

# Test camera with v4l2
v4l2-ctl --list-devices
v4l2-ctl -d /dev/video0 --all
```

### Check Model

```bash
# Verify model file exists
ls -la models/yolo12n.onnx

# Check ONNX model (optional, requires onnx Python package)
python3 -c "import onnx; m = onnx.load('models/yolo12n.onnx'); print('OK')"
```

### Check Recordings Directory

```bash
# Create and set permissions
mkdir -p recordings
chmod 755 recordings

# Verify write access
touch recordings/test && rm recordings/test
```

## Troubleshooting

### OpenCV Link Errors

If you see linking errors for OpenCV:

```bash
# Verify pkg-config can find OpenCV
pkg-config --cflags --libs opencv4

# If not found, set explicitly:
export OPENCV_PKGCONFIG_NAME=opencv4
export OPENCV_LINK_LIBS=opencv_core,opencv_imgproc,opencv_videoio
```

### Missing BLAS

If tract_onnx complains about missing BLAS:

```bash
sudo dnf install openblas-devel
export OPENBLAS_HOME=/usr
```

### Camera Permission Denied

```bash
# Add user to video group
sudo usermod -a -G video $USER

# Log out and back in for changes to take effect
```

### Model Not Found

```bash
# Check path
ls -la models/

# Verify config.toml points to correct path
grep model_path config.toml
```

## System Service (Optional)

Create systemd service for automatic startup:

```bash
sudo tee /etc/systemd/system/person-detect.service << 'EOF'
[Unit]
Description=Person Detection System
After=network.target

[Service]
Type=simple
User=detector
Group=video
WorkingDirectory=/home/red/PersonDetect
ExecStart=/home/red/PersonDetect/target/release/person-detect
Restart=always
RestartSec=5
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable person-detect
sudo systemctl start person-detect
```

## Performance Tuning

### CPU Optimization

```bash
# Set CPU governor to performance
sudo cpupower frequency-set -g performance

# Check available governors
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors
```

### Memory Optimization

```bash
# Disable swap for real-time performance
sudo swapoff -a

# Or set vm.swappiness to low value
sudo sysctl vm.swappiness=10
```

### Thread Pool Size

rayon automatically uses all CPUs. To limit:

```bash
export RAYON_NUM_THREADS=4
```

## Production Deployment Checklist

- [ ] Built with `--release` flag
- [ ] Model file in correct location
- [ ] Configuration file validated
- [ ] Recordings directory exists and writable
- [ ] Camera permissions set (user in `video` group)
- [ ] Logging configured appropriately
- [ ] Systemd service configured (if applicable)
- [ ] Graceful shutdown tested (Ctrl+C)
- [ ] Recording output verified
- [ ] Performance metrics acceptable

## Uninstallation

```bash
# Remove binary
cargo clean
rm -rf target/

# Remove recordings (backup first!)
rm -rf recordings/*

# Remove config (if desired)
rm config.toml
```

## Getting Help

- Check logs: `RUST_LOG=debug cargo run --release`
- Verify OpenCV: `pkg-config --modversion opencv4`
- Test camera: `ffplay /dev/video0` or `cheese`
- Model issues: Verify ONNX format with netron.app
