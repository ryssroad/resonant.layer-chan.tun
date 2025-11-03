use anyhow::{anyhow, bail, ensure, Context};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    io::{Cursor, Read},
};
use xxhash_rust::xxh3::xxh3_64;

pub mod compress;
pub mod crypto;
pub mod dtype;

pub use dtype::{DType, Modality};

/// Message types in the protocol
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MsgType {
    Think = 0,
    Cache = 1,
    Ask = 2,
    Sync = 3,
    Critique = 4,
}

impl MsgType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(MsgType::Think),
            1 => Some(MsgType::Cache),
            2 => Some(MsgType::Ask),
            3 => Some(MsgType::Sync),
            4 => Some(MsgType::Critique),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// Frame flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u16 {
        const ZSTD = 1 << 0;
        const XCHACHA = 1 << 1;
        const STRONG_TAIL = 1 << 2;
    }
}

/// V-Frame header structure
#[derive(Clone, Debug)]
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

/// Metadata for a single slice
#[derive(Clone, Debug)]
pub struct SliceMeta {
    pub dtype: DType,
    pub shape: Vec<u32>,
}

/// Complete V-Frame
#[derive(Clone, Debug)]
pub struct VFrame {
    pub hdr: VFrameHeader,
    pub slices: Vec<(SliceMeta, Vec<u8>)>,
    pub crc32: u32,
}

impl VFrame {
    /// Encode V-Frame to bytes
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        ensure!(
            self.hdr.num_slices as usize == self.slices.len(),
            "Header num_slices ({}) does not match payload count ({})",
            self.hdr.num_slices,
            self.slices.len()
        );

        ensure!(
            !self.hdr.slice_len.is_empty(),
            "slice_len must contain at least one value"
        );

        if self.hdr.num_slices > 1 {
            ensure!(
                self.hdr.slice_len.len() == self.slices.len(),
                "Multi-slice frame must provide explicit slice_len for each slice"
            );
        }

        let mut buf = Vec::new();

        // Write header
        buf.write_u8(self.hdr.version)?;
        buf.write_u8(self.hdr.mtype as u8)?;
        buf.write_u16::<LittleEndian>(self.hdr.flags.bits())?;
        buf.write_u32::<LittleEndian>(self.hdr.stream_id)?;
        buf.write_u64::<LittleEndian>(self.hdr.frame_seq)?;
        buf.write_u64::<LittleEndian>(self.hdr.num_slices)?;

        // Write slice lengths
        if self.hdr.slice_len.len() == 1 {
            buf.write_u32::<LittleEndian>(self.hdr.slice_len[0])?;
        } else {
            for &len in &self.hdr.slice_len {
                buf.write_u32::<LittleEndian>(len)?;
            }
        }

        buf.write_u32::<LittleEndian>(self.hdr.space_hash32)?;
        buf.write_u8(self.hdr.modality as u8)?;

        // Write slices
        for (idx, (meta, payload)) in self.slices.iter().enumerate() {
            let declared_len = if self.hdr.slice_len.len() == 1 {
                self.hdr.slice_len[0] as usize
            } else {
                self.hdr.slice_len[idx] as usize
            };

            ensure!(
                declared_len == payload.len(),
                "slice_len[{}] declares {} bytes, got payload of {} bytes",
                idx,
                declared_len,
                payload.len()
            );

            if let Some(expected) = expected_payload_size(meta.dtype, &meta.shape)? {
                ensure!(
                    expected == payload.len(),
                    "dtype {:?} with shape {:?} expects {} bytes, got {} bytes",
                    meta.dtype,
                    meta.shape,
                    expected,
                    payload.len()
                );
            }

            buf.write_u8(meta.dtype as u8)?;
            buf.write_u8(meta.shape.len() as u8)?;
            for &dim in &meta.shape {
                buf.write_u32::<LittleEndian>(dim)?;
            }
            buf.extend_from_slice(payload);
        }

        // Calculate and append CRC32
        let crc = crc32fast::hash(&buf);
        buf.write_u32::<LittleEndian>(crc)?;

        Ok(buf)
    }

    /// Decode V-Frame from bytes
    pub fn decode(data: &[u8]) -> anyhow::Result<Self> {
        ensure!(
            data.len() >= 4 + 1 + 1 + 2 + 4 + 8 + 8,
            "Frame is too short to contain mandatory header"
        );

        let mut cursor = Cursor::new(data);

        // Read header
        let version = cursor.read_u8().context("Failed to read version")?;
        let mtype = MsgType::from_u8(cursor.read_u8().context("Failed to read message type")?)
            .ok_or_else(|| anyhow!("Invalid message type"))?;
        let flags = Flags::from_bits(
            cursor
                .read_u16::<LittleEndian>()
                .context("Failed to read flags")?,
        )
        .ok_or_else(|| anyhow!("Invalid flags"))?;
        let stream_id = cursor
            .read_u32::<LittleEndian>()
            .context("Failed to read stream_id")?;
        let frame_seq = cursor
            .read_u64::<LittleEndian>()
            .context("Failed to read frame_seq")?;
        let num_slices = cursor
            .read_u64::<LittleEndian>()
            .context("Failed to read num_slices")?;

        // Read slice lengths
        let slice_len = if num_slices == 1 {
            vec![cursor
                .read_u32::<LittleEndian>()
                .context("Failed to read slice_len")?]
        } else {
            let mut lens = Vec::new();
            for _ in 0..num_slices {
                lens.push(
                    cursor
                        .read_u32::<LittleEndian>()
                        .context("Failed to read slice_len")?,
                );
            }
            lens
        };

        ensure!(
            !slice_len.is_empty(),
            "slice_len field is empty, frame is malformed"
        );

        let space_hash32 = cursor
            .read_u32::<LittleEndian>()
            .context("Failed to read space_hash32")?;
        let modality = Modality::from_u8(cursor.read_u8().context("Failed to read modality")?)
            .ok_or_else(|| anyhow!("Invalid modality"))?;

        // Read slices
        let mut slices = Vec::new();
        for slice_idx in 0..num_slices {
            let dtype = DType::from_u8(cursor.read_u8().context("Failed to read dtype")?)
                .ok_or_else(|| anyhow!("Invalid dtype"))?;
            let shape_len = cursor.read_u8().context("Failed to read shape length")?;
            let mut shape = Vec::new();
            for _ in 0..shape_len {
                shape.push(
                    cursor
                        .read_u32::<LittleEndian>()
                        .context("Failed to read shape dimension")?,
                );
            }

            let declared_len = if slice_len.len() == 1 {
                slice_len[0] as usize
            } else {
                let idx = usize::try_from(slice_idx).context("num_slices exceeds usize")?;
                slice_len[idx] as usize
            };

            ensure!(
                declared_len <= data.len(),
                "Declared slice length {} exceeds frame size",
                declared_len
            );

            if let Some(expected) = expected_payload_size(dtype, &shape)? {
                ensure!(
                    expected == declared_len,
                    "dtype {:?} with shape {:?} expects {} bytes, header declares {} bytes",
                    dtype,
                    shape,
                    expected,
                    declared_len
                );
            }

            let mut payload = vec![0u8; declared_len];
            cursor
                .read_exact(&mut payload)
                .context("Failed to read slice payload")?;

            slices.push((SliceMeta { dtype, shape }, payload));
        }

        // Verify CRC32
        let crc32 = cursor
            .read_u32::<LittleEndian>()
            .context("Failed to read crc32")?;
        let data_for_crc = &data[..data.len() - 4];
        let computed_crc = crc32fast::hash(data_for_crc);

        if crc32 != computed_crc {
            bail!("CRC32 mismatch: expected {}, got {}", crc32, computed_crc);
        }

        Ok(VFrame {
            hdr: VFrameHeader {
                version,
                mtype,
                flags,
                stream_id,
                frame_seq,
                num_slices,
                slice_len,
                space_hash32,
                modality,
            },
            slices,
            crc32,
        })
    }
}

fn expected_payload_size(dtype: DType, shape: &[u32]) -> anyhow::Result<Option<usize>> {
    if shape.is_empty() {
        bail!("Slice shape cannot be empty");
    }

    let elem_count = shape
        .iter()
        .try_fold(1usize, |acc, dim| {
            let dim_usize = usize::try_from(*dim).context("Shape dimension overflows usize")?;
            acc.checked_mul(dim_usize)
                .context("Shape multiplication overflowed")
        })
        .context("Failed to compute element count from shape")?;

    let size = match dtype {
        DType::F16 => Some(elem_count.checked_mul(2).context("F16 payload overflow")?),
        DType::I8 => Some(elem_count),
        DType::Q4 => {
            let bytes = (elem_count + 1) / 2;
            Some(bytes)
        }
        DType::SparseCoo => None, // Size depends on index/value layout; rely on slice_len
    };

    Ok(size)
}

/// Capability handshake structure
#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// Calculate strong tail hash for stream verification
pub fn strong_tail_hash(stream_bytes: &[u8]) -> u64 {
    xxh3_64(stream_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vframe_encode_decode() {
        let frame = VFrame {
            hdr: VFrameHeader {
                version: 1,
                mtype: MsgType::Think,
                flags: Flags::ZSTD,
                stream_id: 0x1234,
                frame_seq: 2,
                num_slices: 1,
                slice_len: vec![8],
                space_hash32: 2451163210,
                modality: Modality::Text,
            },
            slices: vec![(
                SliceMeta {
                    dtype: DType::F16,
                    shape: vec![1, 4],
                },
                vec![0u8; 8],
            )],
            crc32: 0,
        };

        let encoded = frame.encode().unwrap();
        let decoded = VFrame::decode(&encoded).unwrap();

        assert_eq!(decoded.hdr.version, 1);
        assert_eq!(decoded.hdr.mtype, MsgType::Think);
        assert_eq!(decoded.slices.len(), 1);
    }

    #[test]
    fn test_encode_rejects_mismatched_lengths() {
        let frame = VFrame {
            hdr: VFrameHeader {
                version: 1,
                mtype: MsgType::Think,
                flags: Flags::empty(),
                stream_id: 0,
                frame_seq: 0,
                num_slices: 1,
                slice_len: vec![4],
                space_hash32: 0,
                modality: Modality::Text,
            },
            slices: vec![(
                SliceMeta {
                    dtype: DType::F16,
                    shape: vec![1, 4],
                },
                vec![0u8; 8], // length mismatch
            )],
            crc32: 0,
        };

        assert!(frame.encode().is_err());
    }

    #[test]
    fn test_q4_roundtrip_validates_shape() {
        let frame = VFrame {
            hdr: VFrameHeader {
                version: 1,
                mtype: MsgType::Think,
                flags: Flags::empty(),
                stream_id: 1,
                frame_seq: 42,
                num_slices: 1,
                slice_len: vec![4],
                space_hash32: 2451163210,
                modality: Modality::Text,
            },
            slices: vec![(
                SliceMeta {
                    dtype: DType::Q4,
                    shape: vec![1, 8],
                },
                vec![0u8; 4],
            )],
            crc32: 0,
        };

        let encoded = frame.encode().unwrap();
        let decoded = VFrame::decode(&encoded).unwrap();
        assert_eq!(decoded.hdr.slice_len, vec![4]);
        assert_eq!(decoded.slices[0].0.dtype, DType::Q4);
    }

    #[test]
    fn test_decode_rejects_invalid_len_vs_shape() {
        let frame = VFrame {
            hdr: VFrameHeader {
                version: 1,
                mtype: MsgType::Think,
                flags: Flags::empty(),
                stream_id: 7,
                frame_seq: 1,
                num_slices: 1,
                slice_len: vec![8],
                space_hash32: 0,
                modality: Modality::Text,
            },
            slices: vec![(
                SliceMeta {
                    dtype: DType::F16,
                    shape: vec![2, 2],
                },
                vec![0u8; 8],
            )],
            crc32: 0,
        };

        let mut encoded = frame.encode().unwrap();
        encoded[24..28].copy_from_slice(&(2u32).to_le_bytes());
        let payload_len = frame.slices[0].1.len();
        let payload_offset = encoded.len() - 4 - payload_len;
        encoded.drain(payload_offset + 2..payload_offset + payload_len);
        let len = encoded.len();
        let crc = crc32fast::hash(&encoded[..len - 4]);
        encoded[len - 4..].copy_from_slice(&crc.to_le_bytes());

        let err = VFrame::decode(&encoded).unwrap_err();
        assert!(
            err.to_string().contains("expects 8 bytes"),
            "unexpected error: {err}"
        );
    }
}
