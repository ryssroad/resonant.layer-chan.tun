# Capability Handshake Specification

## Overview

Before exchanging latent states, peers negotiate capabilities using the Sync message type. This ensures compatibility in embedding spaces, compression, and cryptography.

## Handshake Flow

```
Client                         Server
  |                              |
  |--- (1) Ping ---------------→ |
  |                              |
  |←-- (2) Capability ---------- |
  |                              |
  |--- (3) Think/Cache --------→ |
```

## (1) Client Ping

**V-Frame:**
- Type: `Sync` (3)
- Payload: JSON ping message

**Example JSON:**
```json
{
  "method": "ping",
  "ts": 1730616000
}
```

## (2) Server Capability Response

**V-Frame:**
- Type: `Sync` (3)
- Payload: JSON capability advertisement

**Example JSON:**
```json
{
  "method": "capability",
  "v": 1,
  "agreed_proto": 1,
  "d_model": 4096,
  "embedding_space_id": "universal-llm-v3",
  "space_hash32": 2451163210,
  "compress": ["zstd"],
  "crypto": ["xchacha20poly1305"],
  "supports": {
    "critique": true,
    "dtype": ["f16", "i8", "q4", "sparse"]
  }
}
```

### Fields

| Field               | Type   | Description                                      |
|---------------------|--------|--------------------------------------------------|
| method              | string | Always "capability" for response                 |
| v                   | u32    | Protocol version                                 |
| agreed_proto        | u32    | Negotiated protocol version                      |
| d_model             | u32    | Hidden dimension size                            |
| embedding_space_id  | string | Semantic identifier for embedding space          |
| space_hash32        | u32    | Numeric hash of embedding space (fast check)     |
| compress            | array  | Supported compression algorithms                 |
| crypto              | array  | Supported encryption methods                     |
| supports            | object | Optional features (critique, dtypes, etc.)       |

## Embedding Space Verification

The `embedding_space_id` and `space_hash32` ensure models are compatible:

1. **Semantic ID**: Human-readable identifier (e.g., "universal-llm-v3")
2. **Numeric Hash**: Fast 32-bit discriminator
   - Computed from model architecture + training data signature
   - Mismatches trigger immediate rejection

### Computing space_hash32

```python
import hashlib

def compute_space_hash(space_id: str, model_config: dict) -> int:
    data = f"{space_id}:{model_config['arch']}:{model_config['data_sig']}"
    hash_bytes = hashlib.sha256(data.encode()).digest()
    return int.from_bytes(hash_bytes[:4], 'little')
```

## Compression Negotiation

Supported algorithms:
- `zstd` - Zstandard (recommended)
- `lz4` - LZ4 (faster, lower ratio)
- `none` - No compression

Client uses highest priority algorithm that server supports.

## Crypto Negotiation

Supported methods:
- `xchacha20poly1305` - XChaCha20-Poly1305 AEAD (recommended)
- `aes256gcm` - AES-256-GCM
- `none` - No encryption (trusted networks only)

## Error Handling

### Incompatible Space

**Server Response:**
```json
{
  "method": "error",
  "code": "SPACE_MISMATCH",
  "message": "Embedding space mismatch",
  "expected": "universal-llm-v3",
  "got": "custom-model-v1"
}
```

### Unsupported Features

If client requests unsupported features, server responds with reduced capability set:

```json
{
  "method": "capability",
  "v": 1,
  "agreed_proto": 1,
  "d_model": 4096,
  "compress": ["zstd"],
  "crypto": ["none"],
  "supports": {
    "critique": false,
    "dtype": ["f16"]
  }
}
```

## (3) Data Transfer

After successful handshake, client sends Think/Cache messages with:
- Matching `space_hash32` in header
- Compression/encryption as negotiated
- DTypes from supported set

## Extensions

### Custom Capabilities

Implementations may include custom fields in `supports`:

```json
{
  "supports": {
    "critique": true,
    "dtype": ["f16", "i8"],
    "x-custom-quantization": true,
    "x-streaming-mode": "incremental"
  }
}
```

### Versioning

- `v` field allows future protocol evolution
- `agreed_proto` confirms negotiated version
- Clients should support multiple versions for compatibility

## Security Considerations

1. **Space Hash**: First line of defense against incompatible models
2. **Encryption**: Required for untrusted networks
3. **Authentication**: Out of scope (use TLS, VPN, or application-level auth)

## Example Exchange

```
# Client → Server (Sync/Ping)
Frame: Sync, seq=0, payload={"method":"ping","ts":1730616000}

# Server → Client (Sync/Capability)
Frame: Sync, seq=0, payload={
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

# Client → Server (Think with latent state)
Frame: Think, seq=1, flags=ZSTD, space_hash32=2451163210, payload=[hidden state]
```
