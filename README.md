# 🌀 Resonant Protocol

> **[📖 Russian version / Русская версия](README.ru.md)** | **[🎯 Pitch Deck / Питчфорк](PITCH.ru.md)**

**Model-to-model communication through latent state transfer**

Resonant Protocol enables direct neural network communication by transmitting hidden states, KV-caches, and embeddings. Built on the Kimi K2 specification with extensions for embedding space verification, critique messages, and flexible data types.

---

## 🎯 Features

- **Binary V-Frame format** — Compact, checksummed frames (≤64 KiB)
- **Latent state transfer** — Send hidden representations between models
- **KV-cache sharing** — Efficient attention cache transmission
- **Critique messages** — Model-to-model divergence feedback
- **Embedding space verification** — Ensure compatible representations
- **Compression & encryption** — Zstd + XChaCha20-Poly1305
- **Multiple dtypes** — F16, I8, Q4, Sparse COO
- **Multimodal** — Text, image, audio, graph, mixed

---

## 📦 Repository Structure

```
resonant-protocol/
├── README.md
├── specs/
│   ├── vframe_binary_layout.md      # Binary format specification
│   └── capability_handshake.md      # Handshake protocol
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   # Core V-Frame implementation
│       ├── dtype.rs                 # Data types & modalities
│       ├── compress.rs              # Zstd compression
│       ├── crypto.rs                # XChaCha20-Poly1305
│       └── examples/
│           ├── send_think.rs        # UDP sender example
│           └── recv_think.rs        # UDP receiver example
└── python/
    ├── send_think.py                # Python sender
    └── recv_think.py                # Python receiver
```

---

## 🚀 Quick Start

### Rust Implementation

```bash
cd rust

# Build library
cargo build --release

# Run receiver (in one terminal)
cargo run --example recv_think

# Run sender (in another terminal)
cargo run --example send_think
```

### Python Implementation

```bash
cd python

# Run receiver
python recv_think.py

# In another terminal, send messages
python send_think.py
```

---

## 💡 Usage Examples

### Rust: Sending a Think Message

```rust
use resonant_protocol::*;

let frame = VFrame {
    hdr: VFrameHeader {
        version: 1,
        mtype: MsgType::Think,
        flags: Flags::ZSTD,
        stream_id: 0x1234,
        frame_seq: 1,
        num_slices: 1,
        slice_len: vec![4096],
        space_hash32: 2451163210,
        modality: Modality::Text,
    },
    slices: vec![(
        SliceMeta {
            dtype: DType::F16,
            shape: vec![1, 2048],
        },
        vec![0u8; 4096], // Your hidden state
    )],
    crc32: 0,
};

let bytes = frame.encode()?;
// Send bytes via UDP/TCP/WebRTC
```

### Python: Sending a Critique Message

```python
from send_think import build_frame, MSG_CRITIQUE, DTYPE_F16

divergence = b"\x00" * 32
explanation = json.dumps({"note": "divergence at layer 12"}).encode()

frame = build_frame(
    msg_type=MSG_CRITIQUE,
    stream_id=0x5678,
    frame_seq=3,
    slices=[
        {"dtype": DTYPE_F16, "shape": [16], "payload": divergence},
        {"dtype": DTYPE_I8, "shape": [len(explanation)], "payload": explanation}
    ]
)

sock.sendto(frame, ("127.0.0.1", 50051))
```

---

## 🧪 Message Types

### Think (0)
**Purpose:** Transfer latent states / hidden representations

**Slices:**
- `[0]`: Hidden state (f16/i8/q4)

**Example:** After processing text, Model A sends its hidden state to Model B for continued generation.

### Cache (1)
**Purpose:** Share KV-cache for attention

**Slices:**
- `[0]`: Keys (f16)
- `[1]`: Values (f16)

**Example:** Transfer cached attention for efficient multi-turn dialogue.

### Ask (2)
**Purpose:** Query or request from another model

**Slices:**
- `[0]`: Query embedding
- `[1]` (optional): Context

### Sync (3)
**Purpose:** Control messages, capability handshake

**Slices:**
- `[0]`: JSON payload (ping/capability/error)

**Example:** Negotiate embedding space compatibility before data transfer.

### Critique (4)
**Purpose:** Send divergence feedback or corrections

**Slices:**
- `[0]`: Divergence vector (f16)
- `[1]` (optional): Problematic dimension mask
- `[2]` (optional): Human-readable explanation (JSON)

**Example:** Model B detects semantic drift and sends correction signal to Model A.

---

## 🔐 Security & Compression

### Zstd Compression

Enable with `Flags::ZSTD`:

```rust
// Rust
use resonant_protocol::compress::*;
let compressed = zstd_compress(&data, 3)?;
```

```python
# Python
import zstd
compressed = zstd.compress(data, 3)
```

### XChaCha20-Poly1305 Encryption

Enable with `Flags::XCHACHA`:

```rust
// Rust
use resonant_protocol::crypto::*;
let key = [42u8; 32];
let nonce = [13u8; 24];
let ciphertext = seal_xchacha(&key, &nonce, &plaintext)?;
```

---

## 🧩 Extensions to Kimi K2

### 1. Embedding Space Verification

**Problem:** Models with different architectures produce incompatible embeddings.

**Solution:** Include `space_hash32` in header and capability handshake.

```rust
VFrameHeader {
    space_hash32: 2451163210, // Computed from model signature
    // ...
}
```

Receivers reject frames with mismatched hashes.

### 2. Critique Messages

**Problem:** No feedback mechanism for divergence or semantic drift.

**Solution:** New message type (4) for model-to-model corrections.

```json
{
  "note": "divergence at dims [3, 17]",
  "magnitude": 0.042,
  "suggested_correction": "increase attention to token 15"
}
```

### 3. Extended DTypes

| DType | K2 | Resonant |
|-------|:--:|:--------:|
| F16   | ✅ | ✅       |
| I8    | ❌ | ✅       |
| Q4    | ❌ | ✅       |
| SparseCoo | ❌ | ✅   |

Enables quantized and sparse model communication.

### 4. Modality Field

Support for multimodal models:
- Text (0)
- Image (1)
- Audio (2)
- Graph (3)
- Mixed (4)

### 5. Strong Tail Hash

`xxhash3_64` for full stream verification:

```rust
let stream_hash = strong_tail_hash(&all_frames);
// Include in TAIL frame
```

---

## 📚 Specifications

- **[V-Frame Binary Layout](specs/vframe_binary_layout.md)** — Complete format reference
- **[Capability Handshake](specs/capability_handshake.md)** — Negotiation protocol

---

## 🛠️ Development

### Run Tests

```bash
cd rust
cargo test
```

### Build for Release

```bash
cargo build --release
# Binary in target/release/
```

### Benchmark

```bash
cargo bench
```

---

## 🗺️ Roadmap

- [x] V-Frame encode/decode
- [x] UDP examples (Rust + Python)
- [x] Compression (zstd)
- [x] Encryption (XChaCha20-Poly1305)
- [x] Critique messages
- [x] Embedding space verification
- [ ] TCP transport layer
- [ ] WebRTC data channel adapter
- [ ] FEC (Reed-Solomon)
- [ ] Adaptive quantization (on-the-fly Q4)
- [ ] Stream multiplexing
- [ ] QUIC-based transport

---

## 📖 Use Cases

### 1. Distributed Inference

Split large model across multiple machines:
- Machine A: Layers 1-12
- Machine B: Layers 13-24

```
A: Input → Think(layers 1-12) → [latent state]
   [UDP transfer]
B: Think(layers 13-24) → Output
```

### 2. Model Ensemble

Multiple specialized models collaborate:

```
Vision Model → Think(image features)
   ↓
Text Model → Think(caption + context)
   ↓
Fusion Model → Final output
```

### 3. Continual Learning

Model A learns from Model B's corrections:

```
A: Generate output
B: Critique(divergence in reasoning)
A: Update based on critique
```

### 4. Semantic Routing

Route requests to best model based on embedding space:

```
Router: Ask(query embedding)
   ↓
Specialized Model A/B/C: Think(answer)
```

---

## 🤝 Contributing

Contributions welcome! Areas to explore:

- Transport layer implementations
- Language bindings (Go, Julia, JavaScript)
- Integration with ML frameworks (PyTorch, JAX)
- FEC and error correction
- Performance benchmarks

---

## 📄 License

MIT License - See LICENSE file

---

## 🙏 Acknowledgments

- Based on [Kimi K2 specification](https://github.com/MoonshotAI/Kimi-k2-protocol)
- Inspired by research in model-to-model communication
- Built for open AI infrastructure

---

**Built with ❤️ for the future of distributed intelligence**
