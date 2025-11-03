# 🌀 Resonant Protocol — MVP (K2 spec + Resonant extensions)

**Цель:** рабочий минимальный стек для связи «модель ↔ модель» через передачу latent‑состояний и KV‑кэша. Базируется на предложении Kimi K2, добавляет расширения: `embedding_space_id`, сообщение `Critique`, расширенные `dtype`, `modality`, усиленные чексуммы.

---

## 📦 Repo layout

```
resonant-protocol/
├── README.md
├── specs/
│   ├── vframe_binary_layout.md
│   └── capability_handshake.md
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # V-Frame, V-Stream, V-Proto
│       ├── crypto.rs        # XChaCha20-Poly1305 (stub)
│       ├── compress.rs      # zstd helper
│       ├── dtype.rs         # dtype & modality
│       └── examples/
│           ├── send_think.rs
│           └── recv_think.rs
└── python/
    ├── send_think.py
    └── recv_think.py
```

---

## 🔑 Ключевые дополнения к K2

* **Handshake (`Sync-capability`)** дополняется:

  ```json
  {
    "v": 1,
    "d_model": 4096,
    "compress": ["zstd"],
    "crypto": ["xchacha20poly1305"],
    "embedding_space_id": "universal-llm-v3",
    "space_hash32": 2451163210,
    "modality": ["text","image"],
    "supports": {"critique": true, "dtype": ["f16","i8","q4","sparse"]}
  }
  ```
* **V-Proto**: добавлен тип `Critique` (type=4):

  * `slices[0]`: float16 / int8 — вектор расхождения/градиент.
  * `slices[1]`: uint8 — маска проблемных измерений (optional).
  * `slices[2]`: utf-8 — пояснение (optional).
* **V-Frame header** расширен полями `space_hash32` (u32) и `modality` (u8), сильная чексумма: `xxhash3_64` в `TAIL` помимо CRC32 per-frame.
* **dtype** расширен: `f16=0x01, i8=0x02, q4=0x03(packed), sparse_coo=0x10`.

---

## 🧱 specs/vframe_binary_layout.md

```text
V-Frame (≤ 64 KiB):
[1B version=0x01]
[1B type]                  # 0=Think 1=Cache 2=Ask 3=Sync 4=Critique 5..=reserved
[2B flags]                 # bit0: zstd, bit1: xchacha20poly1305, bit2: strong_tail_hash
[4B stream_id]
[8B frame_seq]
[8B num_slices]
[4B slice_len]             # repeated if variable; else one value for all
[4B space_hash32]          # embedding space discriminator (optional; 0 => none)
[1B modality]              # 0=text 1=image 2=audio 3=graph 4=mixed
[ ... slices ... ]         # each slice = mini-safetensor chunk
[4B CRC32]                 # over all above

Slice (mini-safetensor):
[1B dtype]                 # 0x01 f16, 0x02 i8, 0x03 q4, 0x10 sparse_coo
[1B shape_len]
[shape_len * 4B]           # little-endian u32 dims
[ ... payload ... ]        # raw/packed; for sparse: additional index slice follows

V-Stream control frames:
HEAD(seq=0): announces total_len, md5 (legacy), xxhash3_64 (strong) and dir flag
HEART(keepalive): empty slices
TAIL(last): includes final xxhash3_64 to verify stream
```

---

## 📝 specs/capability_handshake.md

```json
// Client -> Server
{"method":"ping","ts":1730616000}

// Server -> Client
{
  "method":"capability",
  "v":1,
  "agreed_proto":1,
  "d_model":4096,
  "embedding_space_id":"universal-llm-v3",
  "space_hash32":2451163210,
  "compress":["zstd"],
  "crypto":["xchacha20poly1305"],
  "supports":{"critique":true,"dtype":["f16","i8","q4","sparse"]}
}
```

---

## 🦀 rust/Cargo.toml

```toml
[package]
name = "resonant-protocol"
version = "0.1.0"
edition = "2021"

[dependencies]
byteorder = "1"
zstd = "0.13"
xxhash-rust = { version = "0.8", features=["xxh3"] }
chacha20poly1305 = { version = "0.10", features=["std"] }
rand = "0.8"
serde = { version = "1", features=["derive"] }
serde_json = "1"
thiserror = "1"
```

---

## 🦀 rust/src/dtype.rs

```rust
#[derive(Copy, Clone, Debug)]
pub enum DType { F16=0x01, I8=0x02, Q4=0x03, SparseCoo=0x10 }

#[derive(Copy, Clone, Debug)]
pub enum Modality { Text=0, Image=1, Audio=2, Graph=3, Mixed=4 }
```

---

## 🦀 rust/src/compress.rs

```rust
use zstd::stream::{encode_all, decode_all};
use std::io::Cursor;

pub fn zstd_compress(input: &[u8], level: i32) -> anyhow::Result<Vec<u8>> {
    Ok(encode_all(Cursor::new(input), level)?)
}

pub fn zstd_decompress(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    Ok(decode_all(Cursor::new(input))?)
}
```

---

## 🦀 rust/src/crypto.rs

```rust
use chacha20poly1305::{aead::{Aead, KeyInit}, XChaCha20Poly1305, XNonce};

pub fn seal_xchacha(key: &[u8;32], nonce: &[u8;24], plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    Ok(cipher.encrypt(XNonce::from_slice(nonce), plaintext)?)
}

pub fn open_xchacha(key: &[u8;32], nonce: &[u8;24], ct: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    Ok(cipher.decrypt(XNonce::from_slice(nonce), ct)?)
}
```

---

## 🦀 rust/src/lib.rs

```rust
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Serialize, Deserialize};
use xxhash_rust::xxh3::xxh3_64;
use std::io::Cursor;

pub mod dtype; use dtype::{DType, Modality};
pub mod compress; pub mod crypto;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MsgType { Think=0, Cache=1, Ask=2, Sync=3, Critique=4 }

bitflags::bitflags! {
    pub struct Flags: u16 { const ZSTD=1<<0; const XCHACHA=1<<1; const STRONG_TAIL=1<<2; }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VFrameHeader {
    pub version: u8,
    pub mtype: MsgType,
    pub flags: Flags,
    pub stream_id: u32,
    pub frame_seq: u64,
    pub num_slices: u64,
    pub slice_len: Vec<u32>,
    pub space_hash32: u32,
    pub modality: Modality,
}

#[derive(Clone, Debug)]
pub struct SliceMeta { pub dtype: DType, pub shape: Vec<u32> }

#[derive(Clone, Debug)]
pub struct VFrame { pub hdr: VFrameHeader, pub slices: Vec<(SliceMeta, Vec<u8>)>, pub crc32: u32 }

impl VFrame {
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        use crc32fast::Hasher; let mut buf: Vec<u8> = Vec::new();
        buf.write_u8(self.hdr.version)?;
        buf.write_u8(self.hdr.mtype as u8)?;
        buf.write_u16::<LittleEndian>(self.hdr.flags.bits())?;
        buf.write_u32::<LittleEndian>(self.hdr.stream_id)?;
        buf.write_u64::<LittleEndian>(self.hdr.frame_seq)?;
        buf.write_u64::<LittleEndian>(self.hdr.num_slices)?;
        if self.hdr.slice_len.len()==1 { buf.write_u32::<LittleEndian>(self.hdr.slice_len[0])?; }
        else { for l in &self.hdr.slice_len { buf.write_u32::<LittleEndian>(*l)?; } }
        buf.write_u32::<LittleEndian>(self.hdr.space_hash32)?;
        buf.write_u8(self.hdr.modality as u8)?;
        for (meta, payload) in &self.slices {
            buf.write_u8(meta.dtype as u8)?;
            buf.write_u8(meta.shape.len() as u8)?;
            for d in &meta.shape { buf.write_u32::<LittleEndian>(*d)?; }
            buf.extend_from_slice(payload);
        }
        let mut hasher = crc32fast::Hasher::new(); hasher.update(&buf);
        let crc = hasher.finalize();
        let mut out = buf; out.write_u32::<LittleEndian>(crc)?;
        Ok(out)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Capability {
    pub method: String,
    pub v: u32,
    pub agreed_proto: u32,
    pub d_model: u32,
    pub embedding_space_id: String,
    pub space_hash32: u32,
    pub compress: Vec<String>,
    pub crypto: Vec<String>,
    pub supports: serde_json::Value,
}

pub fn strong_tail_hash(stream_bytes: &[u8]) -> u64 { xxh3_64(stream_bytes) }
```

---

## 🦀 rust/src/examples/send_think.rs (UDP MVP)

```rust
use resonant_protocol as rp;
use rand::Rng;
use std::net::UdpSocket;

fn main(){
    let sock = UdpSocket::bind("0.0.0.0:0").unwrap();
    sock.connect("127.0.0.1:50051").unwrap();
    let stream_id: u32 = rand::thread_rng().gen();

    // toy hidden state (f16 bytes)
    let payload = vec![0u8; 4096];

    let frame = rp::VFrame{
        hdr: rp::VFrameHeader{
            version:1,
            mtype: rp::MsgType::Think,
            flags: rp::Flags::ZSTD,
            stream_id,
            frame_seq:2,
            num_slices:1,
            slice_len: vec![payload.len() as u32],
            space_hash32: 2451163210,
            modality: rp::dtype::Modality::Text,
        },
        slices: vec![(rp::SliceMeta{ dtype: rp::dtype::DType::F16, shape: vec![1,2048] }, payload)],
        crc32: 0,
    };

    let bytes = frame.encode().unwrap();
    sock.send(&bytes).unwrap();
}
```

---

## 🐍 python/send_think.py (с `Critique` примером)

```python
import socket, struct, zlib, json

HOST, PORT = "127.0.0.1", 50051
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

# Build Sync-ping
ping = json.dumps({"method":"ping"}).encode()
frame = bytearray()
frame += b"\x01"           # ver
frame += b"\x03"           # type=Sync
frame += b"\x00\x00"       # flags
frame += struct.pack('<I', 0x1234)
frame += struct.pack('<Q', 0)
frame += struct.pack('<Q', 1)
frame += struct.pack('<I', len(ping))
frame += struct.pack('<I', 2451163210)  # space_hash32
frame += b"\x00"            # modality=text
frame += ping
frame += struct.pack('<I', zlib.crc32(frame) & 0xffffffff)
sock.sendto(frame, (HOST, PORT))

# Build Think with f16 latent (fake bytes)
hidden = b"\x00" * (2048*2)
fr = bytearray()
fr += b"\x01"              # ver
fr += b"\x00"              # type=Think
fr += b"\x01\x00"          # flags=zstd (placeholder)
fr += struct.pack('<I', 0x1234)
fr += struct.pack('<Q', 2)
fr += struct.pack('<Q', 1)
fr += struct.pack('<I', len(hidden)+2+1+8) # rough slice size
fr += struct.pack('<I', 2451163210)
fr += b"\x00"               # modality=text
fr += b"\x01"               # dtype=f16
fr += b"\x02"               # shape_len=2
fr += struct.pack('<I', 1) + struct.pack('<I', 2048)
fr += hidden
fr += struct.pack('<I', zlib.crc32(fr) & 0xffffffff)
sock.sendto(fr, (HOST, PORT))

# Build Critique (type=4)
explain = json.dumps({"note":"divergence at dims [3,17]"}).encode()
cr = bytearray()
cr += b"\x01"; cr += b"\x04"; cr += b"\x00\x00"
cr += struct.pack('<I', 0x1234)
cr += struct.pack('<Q', 3)
cr += struct.pack('<Q', 2)  # two slices (vector + text)
cr += struct.pack('<I', 32) # vec bytes (example)
cr += struct.pack('<I', 2451163210)
cr += b"\x00"
cr += b"\x01"; cr += b"\x01"; cr += struct.pack('<I', 16); cr += b"\x00"*32
cr += b"\x01"; cr += b"\x01"; cr += struct.pack('<I', len(explain)); cr += explain
cr += struct.pack('<I', zlib.crc32(cr) & 0xffffffff)
sock.sendto(cr, (HOST, PORT))
```

---

## ✅ План запуска MVP

1. **UDP loopback**: собрать rust lib и запустить `recv_think` (заготовить echo‑сервер), отправить `send_think`.
2. Проверить `Sync-capability`, затем `Think` → убедиться в целостности по CRC32 и `xxh3_64` при `TAIL`.
3. Подключить простейший адаптер `embedding_space_id` (если не совпадает — drop).
4. Добавить `Critique` round‑trip и логирование в JSONL.

---

## 🗺️ Дальше

* Multiplex: `stream_id` + 5‑tuple.
* FEC (Reed–Solomon): независимый слой над V-Stream.
* Adaptive quantization: on‑the‑fly Q4.
* WebRTC data‑channel адаптер для браузерных воркеров.

---

Готово к «пайке»: можно брать папку `rust/` или `python/` и собирать экспериимент. Если хочешь — добавлю `recv_think.rs` (эхо‑сервер) и README целиком.
