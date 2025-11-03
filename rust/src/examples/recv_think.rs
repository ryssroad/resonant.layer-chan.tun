use resonant_protocol::*;
use std::net::UdpSocket;

fn main() -> anyhow::Result<()> {
    println!("🌀 Resonant Protocol - Receiver");
    println!("📡 Listening on 127.0.0.1:50051...\n");

    let sock = UdpSocket::bind("127.0.0.1:50051")?;
    let mut buf = vec![0u8; 65536]; // 64 KiB max frame size

    let mut frame_count = 0;

    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, src)) => {
                frame_count += 1;
                println!("📥 Received {} bytes from {}", size, src);

                // Decode V-Frame
                match VFrame::decode(&buf[..size]) {
                    Ok(frame) => {
                        print_frame_info(&frame, frame_count);

                        // Handle different message types
                        match frame.hdr.mtype {
                            MsgType::Sync => handle_sync(&frame)?,
                            MsgType::Think => handle_think(&frame)?,
                            MsgType::Critique => handle_critique(&frame)?,
                            MsgType::Cache => println!("   [Cache message]"),
                            MsgType::Ask => println!("   [Ask message]"),
                        }

                        // Verify strong hash if flag is set
                        if frame.hdr.flags.contains(Flags::STRONG_TAIL) {
                            let hash = strong_tail_hash(&buf[..size]);
                            println!("   Strong hash: {:#x}", hash);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to decode frame: {}", e);
                    }
                }

                println!();
            }
            Err(e) => {
                eprintln!("❌ Recv error: {}", e);
            }
        }
    }
}

fn print_frame_info(frame: &VFrame, count: usize) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📦 Frame #{}", count);
    println!("   Version: {}", frame.hdr.version);
    println!("   Type: {:?}", frame.hdr.mtype);
    println!("   Stream ID: {:#x}", frame.hdr.stream_id);
    println!("   Sequence: {}", frame.hdr.frame_seq);
    println!("   Slices: {}", frame.hdr.num_slices);
    println!("   Space hash: {:#x}", frame.hdr.space_hash32);
    println!("   Modality: {:?}", frame.hdr.modality);
    println!("   Flags: {:?}", frame.hdr.flags);
    println!("   CRC32: {:#x} ✓", frame.crc32);
}

fn handle_sync(frame: &VFrame) -> anyhow::Result<()> {
    println!("   [Sync-capability handshake]");

    if !frame.slices.is_empty() {
        let (_, payload) = &frame.slices[0];
        match serde_json::from_slice::<serde_json::Value>(payload) {
            Ok(json) => {
                println!("   Payload: {}", serde_json::to_string_pretty(&json)?);

                // Send capability response (in real implementation)
                let response = Capability {
                    method: "capability".to_string(),
                    v: 1,
                    agreed_proto: 1,
                    d_model: 4096,
                    embedding_space_id: "universal-llm-v3".to_string(),
                    space_hash32: 2451163210,
                    compress: vec!["zstd".to_string()],
                    crypto: vec!["xchacha20poly1305".to_string()],
                    supports: serde_json::json!({
                        "critique": true,
                        "dtype": ["f16", "i8", "q4", "sparse"]
                    }),
                };
                println!(
                    "   → Would respond with: {}",
                    serde_json::to_string_pretty(&response)?
                );
            }
            Err(e) => println!("   ⚠️  Could not parse JSON: {}", e),
        }
    }

    Ok(())
}

fn handle_think(frame: &VFrame) -> anyhow::Result<()> {
    println!("   [Think message - latent state]");

    for (idx, (meta, payload)) in frame.slices.iter().enumerate() {
        println!(
            "   Slice {}: {:?} shape={:?} size={} bytes",
            idx,
            meta.dtype,
            meta.shape,
            payload.len()
        );
    }

    // In real implementation: process latent state
    println!("   → Latent state received and ready for processing");

    Ok(())
}

fn handle_critique(frame: &VFrame) -> anyhow::Result<()> {
    println!("   [Critique message]");

    if frame.slices.len() >= 2 {
        let (vec_meta, vec_payload) = &frame.slices[0];
        println!(
            "   Divergence vector: {:?} shape={:?} size={}",
            vec_meta.dtype,
            vec_meta.shape,
            vec_payload.len()
        );

        let (_, explain_payload) = &frame.slices[1];
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(explain_payload) {
            println!("   Explanation: {}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}
