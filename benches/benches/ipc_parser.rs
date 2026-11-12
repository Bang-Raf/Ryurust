//! Benchmarks for `ryu-ipc-parser`: `parse_ipc_message` on a realistic CMIF request
//! (PID + copy handle, pointer/send/receive descriptors, SFCI payload, receive list) and a
//! TIPC request, plus `serialize_cmif` / `serialize_tipc` for typical responses.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_ipc_parser::{parse_ipc_message, serialize_cmif, serialize_tipc, ParsedIpcMessage};
use std::hint::black_box;
use std::time::Duration;

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// CMIF Request (type 4): handle descriptor with PID and one copy handle, one pointer
/// buffer (X), one send buffer (A), one receive buffer (B), 64 bytes of raw data
/// (16-byte alignment pad + SFCI header + 16 bytes of args + trailing pad), one receive
/// list entry (C).
fn build_cmif_request() -> Vec<u8> {
    let mut b = Vec::with_capacity(128);
    let word0 = 4 | (1 << 16) | (1 << 20) | (1 << 24);
    let raw_words = 64 / 4;
    let word1 = raw_words | (2 << 10) | (1 << 31);
    push_u32(&mut b, word0);
    push_u32(&mut b, word1);
    // Handle descriptor: has_pid, 1 copy handle, 0 move handles.
    push_u32(&mut b, 1 | (1 << 1));
    push_u64(&mut b, 0x0000_0000_0000_0051); // pid
    push_u32(&mut b, 0x0000_d00d); // copy handle
                                   // Pointer descriptor: index 0, size 0x100, position 0x8_0001_2000.
    push_u32(&mut b, (0x100 << 16) | (0x8 << 12));
    push_u32(&mut b, 0x0001_2000);
    // Send buffer descriptor: size 0x2000, position 0x9000_0000, flags 0.
    push_u32(&mut b, 0x2000);
    push_u32(&mut b, 0x9000_0000);
    push_u32(&mut b, 0);
    // Receive buffer descriptor: size 0x4000, position 0x9100_0000, flags 0.
    push_u32(&mut b, 0x4000);
    push_u32(&mut b, 0x9100_0000);
    push_u32(&mut b, 0);
    assert_eq!(b.len(), 56);
    // Raw data: 8 bytes of alignment pad (to offset 64), SFCI header, args, then pad.
    b.extend_from_slice(&[0u8; 8]);
    b.extend_from_slice(b"SFCI");
    push_u32(&mut b, 1); // version
    push_u32(&mut b, 3); // command id
    push_u32(&mut b, 0); // token
    push_u64(&mut b, 0x0100_0000_0000_1234); // arg0
    push_u64(&mut b, 0x0000_0000_0000_0001); // arg1
    b.extend_from_slice(&[0u8; 24]);
    assert_eq!(b.len(), 120);
    // Receive list entry: size 0x50, position 0xB000_0000.
    push_u64(&mut b, (0x50u64 << 48) | 0xB000_0000);
    b
}

/// TIPC request (type 0x10 + cmd 3): two send buffers, 16 bytes of raw data.
fn build_tipc_request() -> Vec<u8> {
    let mut b = Vec::with_capacity(48);
    push_u32(&mut b, 0x13 | (2 << 20));
    push_u32(&mut b, 16 / 4);
    for i in 0..2u32 {
        push_u32(&mut b, 0x1000 * (i + 1));
        push_u32(&mut b, 0x4000_0000 + i * 0x1000);
        push_u32(&mut b, 0);
    }
    push_u64(&mut b, 0x0123_4567_89ab_cdef);
    push_u64(&mut b, 0x0000_0000_0000_0010);
    b
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_parser");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    let cmif = build_cmif_request();
    let parsed = parse_ipc_message(&cmif, 0).expect("CMIF request must parse");
    assert_eq!(parsed.msg_type, 4);
    assert_eq!(parsed.handle_desc.pid, 0x51);
    assert_eq!(parsed.handle_desc.copy_handles[0], 0xd00d);
    assert_eq!(parsed.ptr_buffs[0].position, 0x8_0001_2000);
    assert_eq!(parsed.ptr_buffs[0].size, 0x100);
    assert_eq!(parsed.send_buffs[0].position, 0x9000_0000);
    assert_eq!(parsed.recv_buffs[0].size, 0x4000);
    assert_eq!(parsed.raw_data_offset, 64);
    assert_eq!(&cmif[64..68], b"SFCI");
    assert_eq!(parsed.recv_list_count, 1);
    assert_eq!(parsed.recv_list_buffs[0].position, 0xB000_0000);

    group.throughput(Throughput::Bytes(cmif.len() as u64));
    group.bench_function(BenchmarkId::new("parse", "cmif_request_128B"), |b| {
        b.iter(|| black_box(parse_ipc_message(black_box(&cmif), black_box(0))))
    });

    let tipc = build_tipc_request();
    let tparsed = parse_ipc_message(&tipc, 0).expect("TIPC request must parse");
    assert_eq!(tparsed.send_buff_count, 2);
    assert_eq!(tparsed.raw_data_offset, 32);
    assert_eq!(tparsed.raw_data_length, 16);

    group.throughput(Throughput::Bytes(tipc.len() as u64));
    group.bench_function(BenchmarkId::new("parse", "tipc_request_48B"), |b| {
        b.iter(|| black_box(parse_ipc_message(black_box(&tipc), black_box(0))))
    });

    // CMIF response: one move handle, SFCO header + result + 8 bytes of output.
    let mut resp = ParsedIpcMessage {
        msg_type: 4,
        handle_desc_enabled: 1,
        ..Default::default()
    };
    resp.handle_desc.move_count = 1;
    resp.handle_desc.move_handles[0] = 0x0000_beef;
    let mut resp_raw = Vec::with_capacity(24);
    resp_raw.extend_from_slice(b"SFCO");
    resp_raw.extend_from_slice(&1u32.to_le_bytes());
    resp_raw.extend_from_slice(&0u32.to_le_bytes()); // result
    resp_raw.extend_from_slice(&0u32.to_le_bytes());
    resp_raw.extend_from_slice(&0x1234_5678_9abc_def0u64.to_le_bytes());
    let mut out = vec![0u8; 256];
    let cmif_len = serialize_cmif(&resp, &resp_raw, 0, 0, &mut out).expect("serialize_cmif");
    let reparsed = parse_ipc_message(&out[..cmif_len], 0).expect("serialized CMIF must parse");
    assert_eq!(reparsed.handle_desc.move_handles[0], 0xbeef);
    let off = reparsed.raw_data_offset as usize;
    assert_eq!(&out[off..off + 4], b"SFCO");

    group.throughput(Throughput::Bytes(cmif_len as u64));
    group.bench_function(
        BenchmarkId::new("serialize_cmif", "response_24B_payload"),
        |b| {
            b.iter(|| {
                black_box(serialize_cmif(
                    black_box(&resp),
                    black_box(&resp_raw),
                    black_box(0),
                    black_box(0),
                    black_box(&mut out),
                ))
            })
        },
    );

    let tresp = ParsedIpcMessage {
        msg_type: 0x13,
        ..Default::default()
    };
    let tipc_len = serialize_tipc(&tresp, &resp_raw[8..], &mut out).expect("serialize_tipc");
    group.throughput(Throughput::Bytes(tipc_len as u64));
    group.bench_function(
        BenchmarkId::new("serialize_tipc", "response_16B_payload"),
        |b| {
            b.iter(|| {
                black_box(serialize_tipc(
                    black_box(&tresp),
                    black_box(&resp_raw[8..]),
                    black_box(&mut out),
                ))
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
