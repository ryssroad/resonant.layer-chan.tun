use rand::Rng;
use resonant_protocol::*;
use std::net::UdpSocket;

fn main() -> anyhow::Result<()> {
    println!("🌀 Resonant Protocol - Sending Think message");

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.connect("127.0.0.1:50051")?;

    let stream_id: u32 = rand::thread_rng().gen();
    println!("📡 Stream ID: {:#x}", stream_id);

    // First, send Sync-capability handshake
    println!("\n1️⃣  Sending Sync-capability handshake...");
    let capability = serde_json::json!({
        "method": "ping",
        "ts": 1730616000u64
    });

    let cap_bytes = serde_json::to_vec(&capability)?;
    let sync_frame = VFrame {
        hdr: VFrameHeader {
            version: 1,
            mtype: MsgType::Sync,
            flags: Flags::empty(),
            stream_id,
            frame_seq: 0,
            num_slices: 1,
            slice_len: vec![cap_bytes.len() as u32],
            space_hash32: 2451163210,
            modality: Modality::Text,
        },
        slices: vec![(
            SliceMeta {
                dtype: DType::I8,
                shape: vec![cap_bytes.len() as u32],
            },
            cap_bytes,
        )],
        crc32: 0,
    };

    let sync_bytes = sync_frame.encode()?;
    sock.send(&sync_bytes)?;
    println!("✅ Sent {} bytes", sync_bytes.len());

    // Send Think message with latent state
    println!("\n2️⃣  Sending Think message with f16 latent state...");

    // Create toy hidden state (4096 f16 values = 8192 bytes)
    let hidden_size = 2048;
    let payload = vec![0u8; hidden_size * 2]; // f16 = 2 bytes

    let think_frame = VFrame {
        hdr: VFrameHeader {
            version: 1,
            mtype: MsgType::Think,
            flags: Flags::ZSTD, // Enable compression
            stream_id,
            frame_seq: 1,
            num_slices: 1,
            slice_len: vec![payload.len() as u32],
            space_hash32: 2451163210,
            modality: Modality::Text,
        },
        slices: vec![(
            SliceMeta {
                dtype: DType::F16,
                shape: vec![1, hidden_size as u32],
            },
            payload,
        )],
        crc32: 0,
    };

    let think_bytes = think_frame.encode()?;
    sock.send(&think_bytes)?;
    println!("✅ Sent {} bytes", think_bytes.len());
    println!("   Shape: [1, {}]", hidden_size);
    println!("   DType: F16");

    // Send Critique message
    println!("\n3️⃣  Sending Critique message...");

    let divergence_vec = vec![0u8; 32]; // Small divergence vector
    let explanation = serde_json::json!({
        "note": "divergence at dims [3, 17]",
        "magnitude": 0.042
    });
    let explain_bytes = serde_json::to_vec(&explanation)?;

    let critique_frame = VFrame {
        hdr: VFrameHeader {
            version: 1,
            mtype: MsgType::Critique,
            flags: Flags::empty(),
            stream_id,
            frame_seq: 2,
            num_slices: 2,
            slice_len: vec![divergence_vec.len() as u32, explain_bytes.len() as u32],
            space_hash32: 2451163210,
            modality: Modality::Text,
        },
        slices: vec![
            (
                SliceMeta {
                    dtype: DType::F16,
                    shape: vec![16],
                },
                divergence_vec,
            ),
            (
                SliceMeta {
                    dtype: DType::I8,
                    shape: vec![explain_bytes.len() as u32],
                },
                explain_bytes,
            ),
        ],
        crc32: 0,
    };

    let critique_bytes = critique_frame.encode()?;
    sock.send(&critique_bytes)?;
    println!("✅ Sent {} bytes", critique_bytes.len());

    println!("\n🎉 All messages sent successfully!");
    println!("   Waiting for receiver to process...");

    Ok(())
}
