# ⚡ Quick Start Guide — Resonant Protocol

Get started with model-to-model communication in 5 minutes.

---

## 🎯 Goal

Send a latent state from one process to another using V-Frames over UDP.

---

## 📋 Prerequisites

**Rust:**
- Rust 1.82+ (`rustup` installed)

**Python:**
- Python 3.8+

---

## 🦀 Option 1: Rust (Recommended)

### Step 1: Build

```bash
cd rust
cargo build --release --examples
```

### Step 2: Run Receiver

**Terminal 1:**
```bash
cargo run --release --example recv_think
```

You'll see:
```
🌀 Resonant Protocol - Receiver
📡 Listening on 127.0.0.1:50051...
```

### Step 3: Send Messages

**Terminal 2:**
```bash
cargo run --release --example send_think
```

Output:
```
🌀 Resonant Protocol - Sending Think message
📡 Stream ID: 0xabcd1234

1️⃣  Sending Sync-capability handshake...
✅ Sent 87 bytes

2️⃣  Sending Think message with f16 latent state...
✅ Sent 4152 bytes
   Shape: [1, 2048]
   DType: F16

3️⃣  Sending Critique message...
✅ Sent 123 bytes

🎉 All messages sent successfully!
```

### Step 4: Check Receiver

**Terminal 1** will show:
```
📥 Received 87 bytes from 127.0.0.1:xxxxx
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📦 Frame #1
   Version: 1
   Type: Sync
   Stream ID: 0xabcd1234
   Sequence: 0
   Slices: 1
   Space hash: 0x921e8b5a
   Modality: Text
   Flags: (empty)
   CRC32: 0x... ✓
   [Sync-capability handshake]
   Payload: {
     "method": "ping",
     "ts": 1730616000
   }

📥 Received 4152 bytes from 127.0.0.1:xxxxx
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📦 Frame #2
   Version: 1
   Type: Think
   ...
   [Think message - latent state]
   Slice 0: F16 shape=[1, 2048] size=4096 bytes
   → Latent state received and ready for processing

...
```

✅ **Success!** You've transmitted a latent state between processes.

---

## 🐍 Option 2: Python

### Step 1: Run Receiver

**Terminal 1:**
```bash
cd python
python recv_think.py
```

### Step 2: Send Messages

**Terminal 2:**
```bash
python send_think.py
```

Output is similar to Rust version.

---

## 🧪 What Just Happened?

1. **Sync handshake** — Sender announced its capabilities
2. **Think message** — 2048 f16 values (latent state) transmitted
3. **Critique message** — Divergence feedback sent

All with:
- ✅ CRC32 integrity checks
- ✅ Embedding space verification (`space_hash32`)
- ✅ Binary V-Frame encoding

---

## 🔬 Next Steps

### Explore the Code

**Rust:**
- `rust/src/lib.rs` — V-Frame encode/decode
- `rust/src/examples/send_think.rs` — Sending logic
- `rust/src/examples/recv_think.rs` — Receiving logic

**Python:**
- `python/send_think.py` — Frame builder
- `python/recv_think.py` — Frame decoder

### Modify Parameters

**Change hidden size:**
```rust
// send_think.rs
let hidden_size = 4096; // was 2048
```

**Enable compression:**
```rust
flags: Flags::ZSTD, // Already enabled
```

**Add encryption:**
```rust
flags: Flags::ZSTD | Flags::XCHACHA,
// Need to call crypto::seal_xchacha() on payload
```

### Test Over Network

**Receiver (Machine A):**
```bash
cargo run --example recv_think
# Listening on 127.0.0.1:50051
```

**Sender (Machine B):**
```rust
// Edit send_think.rs
sock.connect("192.168.1.100:50051")?; // Machine A's IP
```

---

## 📚 Learn More

- **[README.md](README.md)** — Full documentation
- **[specs/vframe_binary_layout.md](specs/vframe_binary_layout.md)** — Binary format
- **[specs/capability_handshake.md](specs/capability_handshake.md)** — Handshake protocol

---

## 🐛 Troubleshooting

### "Address already in use"

Another process is using port 50051:
```bash
# Find and kill it
lsof -ti :50051 | xargs kill -9

# Or change port in both send/recv examples
```

### "CRC32 mismatch"

Network corruption detected. This is normal on lossy networks. Future version will add FEC.

### Receiver not seeing messages

Check firewall:
```bash
# macOS
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /path/to/recv_think

# Linux
sudo ufw allow 50051/udp
```

---

## 🎉 You're Ready!

You've successfully transmitted latent states using Resonant Protocol.

Next: Build your distributed inference system! 🚀
