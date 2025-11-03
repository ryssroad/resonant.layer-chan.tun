# V-Frame Binary Layout Specification

## Overview

V-Frame is the fundamental unit of data transmission in Resonant Protocol. Each frame is self-contained and includes checksums for integrity verification.

## Frame Structure

### Maximum Size
- **64 KiB** (65536 bytes) per frame

### V-Frame Header (Variable Length)

```
Offset  Size  Field              Type      Description
------  ----  ----------------   -------   ---------------------------
0       1     version            u8        Protocol version (0x01)
1       1     type               u8        Message type (0-4)
2       2     flags              u16 LE    Feature flags
4       4     stream_id          u32 LE    Stream identifier
8       8     frame_seq          u64 LE    Frame sequence number
16      8     num_slices         u64 LE    Number of data slices
24      4*N   slice_len          u32 LE    Length per slice (N=1 or N=num_slices)
24+4*N  4     space_hash32       u32 LE    Embedding space discriminator
28+4*N  1     modality           u8        Data modality (0-4)
```

### Message Types

| Value | Name     | Description                           |
|-------|----------|---------------------------------------|
| 0     | Think    | Latent state / hidden representation  |
| 1     | Cache    | KV-cache transfer                     |
| 2     | Ask      | Query / request                       |
| 3     | Sync     | Control / capability handshake        |
| 4     | Critique | Divergence feedback / correction      |

### Flags (Bitfield)

| Bit | Name        | Description                    |
|-----|-------------|--------------------------------|
| 0   | ZSTD        | Payload compressed with zstd   |
| 1   | XCHACHA     | Encrypted with XChaCha20-Poly1305 |
| 2   | STRONG_TAIL | Stream includes xxhash3_64 tail |

### Modality

| Value | Name  | Description           |
|-------|-------|-----------------------|
| 0     | Text  | Text/NLP data         |
| 1     | Image | Vision/image data     |
| 2     | Audio | Audio/speech data     |
| 3     | Graph | Structured graph data |
| 4     | Mixed | Multimodal data       |

## Slice Format (Mini-Safetensor)

Each slice contains typed tensor data:

```
Offset  Size    Field      Type      Description
------  ------  ---------  -------   ---------------------------
0       1       dtype      u8        Data type identifier
1       1       shape_len  u8        Number of dimensions
2       4*D     shape      u32 LE    Dimension sizes (D=shape_len)
2+4*D   varies  payload    bytes     Raw tensor data
```

### DType Values

| Value | Name       | Description                    | Size per element |
|-------|------------|--------------------------------|------------------|
| 0x01  | F16        | IEEE 754 half-precision float  | 2 bytes         |
| 0x02  | I8         | Signed 8-bit integer           | 1 byte          |
| 0x03  | Q4         | 4-bit quantized (packed)       | 0.5 bytes       |
| 0x10  | SparseCoo  | Sparse COO format with indices | Variable        |

## CRC32 Checksum

- Appended at the end of each frame
- Computed over all preceding bytes
- Little-endian u32
- Algorithm: CRC32 (IEEE 802.3 polynomial)

```
... [header + slices] ...
[4B CRC32]
```

## V-Stream Control Frames

For multi-frame streams:

### HEAD Frame (seq=0)
- Announces total stream length
- Includes MD5 (legacy) and xxhash3_64 (strong) checksums
- Direction flag for bidirectional streams

### HEART Frame (Keepalive)
- Empty slices (num_slices=0)
- Maintains connection state
- No payload data

### TAIL Frame (Last)
- Final frame in stream
- If `STRONG_TAIL` flag set, includes xxhash3_64 of entire stream
- Enables full stream integrity verification

## Example: Minimal Think Frame

```
01           # version=1
00           # type=Think
01 00        # flags=ZSTD
34 12 00 00  # stream_id=0x1234
02 00 00 00 00 00 00 00  # frame_seq=2
01 00 00 00 00 00 00 00  # num_slices=1
00 10 00 00  # slice_len=4096
AA BB CC DD  # space_hash32
00           # modality=Text

# Slice data
01           # dtype=F16
02           # shape_len=2
01 00 00 00  # shape[0]=1
00 08 00 00  # shape[1]=2048
[4096 bytes of F16 data]

AB CD EF 12  # CRC32
```

## Notes

- All multi-byte integers are **little-endian**
- Compression/encryption applied **per-frame** before CRC
- Space hash discriminator enables embedding space verification
- Sparse formats require additional index slices (not shown)
