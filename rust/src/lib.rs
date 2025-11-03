use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
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
        for (meta, payload) in &self.slices {
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
        let mut cursor = Cursor::new(data);

        // Read header
        let version = cursor.read_u8()?;
        let mtype = MsgType::from_u8(cursor.read_u8()?)
            .ok_or_else(|| anyhow::anyhow!("Invalid message type"))?;
        let flags = Flags::from_bits(cursor.read_u16::<LittleEndian>()?)
            .ok_or_else(|| anyhow::anyhow!("Invalid flags"))?;
        let stream_id = cursor.read_u32::<LittleEndian>()?;
        let frame_seq = cursor.read_u64::<LittleEndian>()?;
        let num_slices = cursor.read_u64::<LittleEndian>()?;

        // Read slice lengths
        let slice_len = if num_slices == 1 {
            vec![cursor.read_u32::<LittleEndian>()?]
        } else {
            let mut lens = Vec::new();
            for _ in 0..num_slices {
                lens.push(cursor.read_u32::<LittleEndian>()?);
            }
            lens
        };

        let space_hash32 = cursor.read_u32::<LittleEndian>()?;
        let modality = Modality::from_u8(cursor.read_u8()?)
            .ok_or_else(|| anyhow::anyhow!("Invalid modality"))?;

        // Read slices
        let mut slices = Vec::new();
        for _ in 0..num_slices {
            let dtype = DType::from_u8(cursor.read_u8()?)
                .ok_or_else(|| anyhow::anyhow!("Invalid dtype"))?;
            let shape_len = cursor.read_u8()?;
            let mut shape = Vec::new();
            for _ in 0..shape_len {
                shape.push(cursor.read_u32::<LittleEndian>()?);
            }

            // Calculate payload size
            let payload_size = shape.iter().product::<u32>() as usize
                * match dtype {
                    DType::F16 => 2,
                    DType::I8 => 1,
                    DType::Q4 => 1, // Packed
                    DType::SparseCoo => 4, // Simplified
                };

            let mut payload = vec![0u8; payload_size];
            cursor.read_exact(&mut payload)?;

            slices.push((SliceMeta { dtype, shape }, payload));
        }

        // Verify CRC32
        let crc32 = cursor.read_u32::<LittleEndian>()?;
        let data_for_crc = &data[..data.len() - 4];
        let computed_crc = crc32fast::hash(data_for_crc);

        if crc32 != computed_crc {
            anyhow::bail!("CRC32 mismatch: expected {}, got {}", crc32, computed_crc);
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
}
