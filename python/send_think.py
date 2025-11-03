#!/usr/bin/env python3
"""
Resonant Protocol - Python sender example
Sends Sync, Think, and Critique messages via UDP
"""

import socket
import struct
import zlib
import json
import random

HOST, PORT = "127.0.0.1", 50051

# Message types
MSG_THINK = 0
MSG_CACHE = 1
MSG_ASK = 2
MSG_SYNC = 3
MSG_CRITIQUE = 4

# Modality
MODALITY_TEXT = 0
MODALITY_IMAGE = 1

# DType
DTYPE_F16 = 0x01
DTYPE_I8 = 0x02
DTYPE_Q4 = 0x03

# Flags
FLAG_ZSTD = 1 << 0
FLAG_XCHACHA = 1 << 1
FLAG_STRONG_TAIL = 1 << 2


def build_frame(
    msg_type: int,
    stream_id: int,
    frame_seq: int,
    slices: list,
    space_hash32: int = 2451163210,
    modality: int = MODALITY_TEXT,
    flags: int = 0,
) -> bytes:
    """Build a V-Frame"""
    frame = bytearray()

    # Header
    frame += struct.pack("B", 1)  # version
    frame += struct.pack("B", msg_type)
    frame += struct.pack("<H", flags)
    frame += struct.pack("<I", stream_id)
    frame += struct.pack("<Q", frame_seq)
    frame += struct.pack("<Q", len(slices))

    # Slice lengths
    if len(slices) == 1:
        frame += struct.pack("<I", len(slices[0]["payload"]))
    else:
        for s in slices:
            frame += struct.pack("<I", len(s["payload"]))

    frame += struct.pack("<I", space_hash32)
    frame += struct.pack("B", modality)

    # Slices
    for s in slices:
        frame += struct.pack("B", s["dtype"])
        frame += struct.pack("B", len(s["shape"]))
        for dim in s["shape"]:
            frame += struct.pack("<I", dim)
        frame += s["payload"]

    # CRC32
    crc = zlib.crc32(frame) & 0xFFFFFFFF
    frame += struct.pack("<I", crc)

    return bytes(frame)


def main():
    print("🌀 Resonant Protocol - Python Sender")

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    stream_id = random.randint(0, 0xFFFFFFFF)

    print(f"📡 Stream ID: {stream_id:#x}\n")

    # 1. Send Sync-capability
    print("1️⃣  Sending Sync-capability...")
    ping_data = json.dumps({"method": "ping", "ts": 1730616000}).encode("utf-8")

    sync_frame = build_frame(
        msg_type=MSG_SYNC,
        stream_id=stream_id,
        frame_seq=0,
        slices=[{"dtype": DTYPE_I8, "shape": [len(ping_data)], "payload": ping_data}],
    )

    sock.sendto(sync_frame, (HOST, PORT))
    print(f"✅ Sent {len(sync_frame)} bytes\n")

    # 2. Send Think with f16 latent
    print("2️⃣  Sending Think message...")
    hidden_size = 2048
    hidden_payload = b"\x00" * (hidden_size * 2)  # f16 = 2 bytes each

    think_frame = build_frame(
        msg_type=MSG_THINK,
        stream_id=stream_id,
        frame_seq=1,
        slices=[
            {"dtype": DTYPE_F16, "shape": [1, hidden_size], "payload": hidden_payload}
        ],
        flags=FLAG_ZSTD,
    )

    sock.sendto(think_frame, (HOST, PORT))
    print(f"✅ Sent {len(think_frame)} bytes")
    print(f"   Shape: [1, {hidden_size}]")
    print(f"   DType: F16\n")

    # 3. Send Critique
    print("3️⃣  Sending Critique message...")
    divergence_vec = b"\x00" * 32
    explanation = json.dumps(
        {"note": "divergence at dims [3, 17]", "magnitude": 0.042}
    ).encode("utf-8")

    critique_frame = build_frame(
        msg_type=MSG_CRITIQUE,
        stream_id=stream_id,
        frame_seq=2,
        slices=[
            {"dtype": DTYPE_F16, "shape": [16], "payload": divergence_vec},
            {"dtype": DTYPE_I8, "shape": [len(explanation)], "payload": explanation},
        ],
    )

    sock.sendto(critique_frame, (HOST, PORT))
    print(f"✅ Sent {len(critique_frame)} bytes\n")

    print("🎉 All messages sent!")
    sock.close()


if __name__ == "__main__":
    main()
