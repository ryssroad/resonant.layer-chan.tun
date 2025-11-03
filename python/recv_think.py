#!/usr/bin/env python3
"""
Resonant Protocol - Python receiver example
Receives and decodes V-Frames via UDP
"""

import socket
import struct
import zlib
import json

HOST, PORT = "127.0.0.1", 50051

# Message types
MSG_TYPES = {
    0: "Think",
    1: "Cache",
    2: "Ask",
    3: "Sync",
    4: "Critique",
}

# DTypes
DTYPES = {
    0x01: "F16",
    0x02: "I8",
    0x03: "Q4",
    0x10: "SparseCoo",
}

# Modalities
MODALITIES = {
    0: "Text",
    1: "Image",
    2: "Audio",
    3: "Graph",
    4: "Mixed",
}


def decode_frame(data: bytes) -> dict:
    """Decode a V-Frame"""
    offset = 0

    # Header
    version = data[offset]
    offset += 1
    msg_type = data[offset]
    offset += 1
    flags = struct.unpack("<H", data[offset : offset + 2])[0]
    offset += 2
    stream_id = struct.unpack("<I", data[offset : offset + 4])[0]
    offset += 4
    frame_seq = struct.unpack("<Q", data[offset : offset + 8])[0]
    offset += 8
    num_slices = struct.unpack("<Q", data[offset : offset + 8])[0]
    offset += 8

    # Slice lengths
    if num_slices == 1:
        slice_lens = [struct.unpack("<I", data[offset : offset + 4])[0]]
        offset += 4
    else:
        slice_lens = []
        for _ in range(num_slices):
            slice_lens.append(struct.unpack("<I", data[offset : offset + 4])[0])
            offset += 4

    space_hash32 = struct.unpack("<I", data[offset : offset + 4])[0]
    offset += 4
    modality = data[offset]
    offset += 1

    # Slices
    slices = []
    for _ in range(num_slices):
        dtype = data[offset]
        offset += 1
        shape_len = data[offset]
        offset += 1
        shape = []
        for _ in range(shape_len):
            shape.append(struct.unpack("<I", data[offset : offset + 4])[0])
            offset += 4

        # Calculate payload size based on dtype and shape
        payload_size = 1
        for dim in shape:
            payload_size *= dim

        # Adjust for dtype size
        if dtype == 0x01:  # F16
            payload_size *= 2
        elif dtype == 0x02:  # I8
            payload_size *= 1
        # For simplicity, use shape product

        # Read payload
        payload = data[offset : offset + payload_size]
        offset += payload_size

        slices.append({"dtype": dtype, "shape": shape, "payload": payload})

    # CRC32
    crc32 = struct.unpack("<I", data[offset : offset + 4])[0]
    computed_crc = zlib.crc32(data[: offset]) & 0xFFFFFFFF

    if crc32 != computed_crc:
        raise ValueError(
            f"CRC32 mismatch: expected {crc32:#x}, got {computed_crc:#x}"
        )

    return {
        "version": version,
        "msg_type": msg_type,
        "flags": flags,
        "stream_id": stream_id,
        "frame_seq": frame_seq,
        "num_slices": num_slices,
        "space_hash32": space_hash32,
        "modality": modality,
        "slices": slices,
        "crc32": crc32,
    }


def print_frame(frame: dict, count: int):
    """Print frame info"""
    print("━" * 40)
    print(f"📦 Frame #{count}")
    print(f"   Version: {frame['version']}")
    print(f"   Type: {MSG_TYPES.get(frame['msg_type'], 'Unknown')}")
    print(f"   Stream ID: {frame['stream_id']:#x}")
    print(f"   Sequence: {frame['frame_seq']}")
    print(f"   Slices: {frame['num_slices']}")
    print(f"   Space hash: {frame['space_hash32']:#x}")
    print(f"   Modality: {MODALITIES.get(frame['modality'], 'Unknown')}")
    print(f"   Flags: {frame['flags']:#x}")
    print(f"   CRC32: {frame['crc32']:#x} ✓")


def handle_sync(frame: dict):
    """Handle Sync message"""
    print("   [Sync-capability handshake]")
    if frame["slices"]:
        payload = frame["slices"][0]["payload"]
        try:
            data = json.loads(payload.decode("utf-8"))
            print(f"   Payload: {json.dumps(data, indent=2)}")
        except Exception as e:
            print(f"   ⚠️  Could not parse: {e}")


def handle_think(frame: dict):
    """Handle Think message"""
    print("   [Think message - latent state]")
    for idx, s in enumerate(frame["slices"]):
        dtype_name = DTYPES.get(s["dtype"], "Unknown")
        print(
            f"   Slice {idx}: {dtype_name} shape={s['shape']} size={len(s['payload'])} bytes"
        )


def handle_critique(frame: dict):
    """Handle Critique message"""
    print("   [Critique message]")
    if len(frame["slices"]) >= 2:
        vec = frame["slices"][0]
        print(
            f"   Divergence: {DTYPES.get(vec['dtype'])} shape={vec['shape']} size={len(vec['payload'])}"
        )

        explain = frame["slices"][1]["payload"]
        try:
            data = json.loads(explain.decode("utf-8"))
            print(f"   Explanation: {json.dumps(data, indent=2)}")
        except Exception:
            pass


def main():
    print("🌀 Resonant Protocol - Python Receiver")
    print(f"📡 Listening on {HOST}:{PORT}...\n")

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((HOST, PORT))

    frame_count = 0

    try:
        while True:
            data, addr = sock.recvfrom(65536)
            frame_count += 1

            print(f"📥 Received {len(data)} bytes from {addr}")

            try:
                frame = decode_frame(data)
                print_frame(frame, frame_count)

                # Handle by type
                msg_type = frame["msg_type"]
                if msg_type == 3:  # Sync
                    handle_sync(frame)
                elif msg_type == 0:  # Think
                    handle_think(frame)
                elif msg_type == 4:  # Critique
                    handle_critique(frame)

                print()

            except Exception as e:
                print(f"❌ Failed to decode: {e}\n")

    except KeyboardInterrupt:
        print("\n👋 Shutting down...")
        sock.close()


if __name__ == "__main__":
    main()
