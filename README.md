# 🌊 WaveBranch

> **High-performance Distributed Audio VCS**

![WaveBranch Demo](assets/wave-demo.gif)

## The Problem
Traditional version control systems like Git were built for text. They treat `.wav` files as opaque, monolithic binary blobs. If two producers edit the same audio track in parallel, Git forces a hard conflict, resulting in massive storage bloat and lost creative work. You cannot conventionally "merge" audio.

## The Solution
**WaveBranch** fundamentally solves this problem algorithmically. By fusing Distributed Data Structures (Merkle DAGs) with Digital Signal Processing (DSP), WaveBranch introduces true multi-track audio branching, diffing, and merging.

When two branches modify the same audio track, WaveBranch leverages algorithmic Phase Cancellation:
1. **Base Extraction**: Identifies the common ancestor (Base).
2. **Delta Isolation**: Inverts the phase of the Base and sums it against Branch A and B to isolate raw changes (e.g., reverb, EQ boosts).
3. **Safe Summation**: Integrates the deltas back onto the Base using safe, zero-cost `wrapping_add` operations without dynamic range clipping.

### Key Features
- **Deterministic Hashing**: Merkle DAG architecture specifically optimized for large audio buffers.
- **Conflict-Free Audio Merging**: Native phase-cancellation algorithm instead of binary overwriting.
- **Zero-Copy Sync**: High-speed P2P networking for studio-to-studio synchronization.
- **Memory Safety**: 100% Rust, ensuring no buffer overflows during complex DSP operations

---

## �️ Requirements
Before installing, ensure you have the following:
* **Rust Toolchain**: [Install Rust](https://rustup.rs/) (Stable version recommended).
* **Audio Drivers**: Windows users may need `ASIO` or standard `WASAPI` drivers for DSP playback.

---

## �🚀 Installation

WaveBranch is built entirely in Rust for maximum cross-platform performance.

```bash
# Clone the repository
git clone https://github.com/DevAlnahari/WaveBranch.git
cd WaveBranch

# Build and install the binary globally via Cargo
cargo install --path .

# Verify the installation
wave --version
```

---

## ⚡ Quick Start
1. **Prepare your workspace**: Drop your `.wav` files into a new folder.
2. **Initialize**:
```bash
wave init
wave add .
wave commit -m "Initial studio acoustic tracking"
```

### 2. Branching & DSP Merging
```bash
# Create and populate an experimental branch
wave branch "reverb-experiment"
wave checkout "reverb-experiment"

wave add .
wave commit -m "Applied heavy hall reverb"

# Switch back and merge using 3-Way DSP Phase Cancellation
wave checkout "main"
wave merge "reverb-experiment"
```

### 3. Chronological Time Travel
```bash
# View the linear timeline
wave log

# Revert workspace to an exact state
wave reset <COMMIT_HASH>
```

### 4. Distributed Synchronous Networking
Host and exchange repositories directly peer-to-peer over an optimized zero-copy TCP stream.
```bash
# Start the synchronous TCP daemon
wave serve -p 8080

# In another studio machine: Clone the entire DAG payload
wave clone 127.0.0.1:8080

# Sync offline changes seamlessly
wave push 127.0.0.1:8080
wave pull 127.0.0.1:8080
```

---

## 📄 License
This project is licensed under the [MIT License](LICENSE).
