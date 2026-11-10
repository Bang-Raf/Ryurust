//! Benchmarks for `ryu-surfaceflinger-queue`: one full triple-buffered frame cycle on
//! `SurfaceFlingerQueue` (dequeue -> queue -> acquire -> release), `find_oldest_free_slot`
//! over 64 free slots, and Binder parcel encode/decode (`ParcelWriter` for a QueueBuffer-style
//! payload, `ParcelReader::read_interface_token`).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ryu_surfaceflinger_queue::{ParcelReader, ParcelWriter, QueueItem, SurfaceFlingerQueue};
use std::hint::black_box;
use std::time::Duration;

const FRAMES: u64 = 256;
const PRODUCER_TOKEN: &str = "android.gui.IGraphicBufferProducer";

struct XorShift64(u64);

impl XorShift64 {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// Runs `n` frames through the queue; returns the next frame number.
fn run_frames(q: &SurfaceFlingerQueue, start_frame: u64, n: u64) -> u64 {
    let mut frame = start_frame;
    for _ in 0..n {
        let slot = q.dequeue_slot(0).expect("free slot");
        let ok = q.queue_buffer(QueueItem {
            slot: slot as i32,
            transform: 0,
            scaling_mode: 1,
            _pad: 0,
            frame_number: frame,
            timestamp: (frame as i64) * 16_666_667,
        });
        debug_assert!(ok);
        let item = q.acquire_buffer().expect("queued item");
        q.release_buffer(item.slot as usize);
        frame += 1;
    }
    frame
}

/// Builds a parcel with the interface token followed by a QueueBuffer input
/// (slot, timestamp, isAutoTimestamp, crop rect, scaling mode, transform, sticky transform,
/// async flag, swap interval, fence).
/// `token_utf16` is the UTF-16LE token including its NUL terminator.
fn build_queue_buffer_parcel(buf: &mut [u8], token_utf16: &[u8]) -> usize {
    let mut w = ParcelWriter::new(buf).expect("parcel buffer");
    w.write_i32(0x100); // strict policy
    w.write_i32((token_utf16.len() / 2 - 1) as i32);
    w.write_bytes_inplace(token_utf16);
    w.write_i32(1); // slot
    w.write_i32(0x54); // flattened size
    w.write_i64(1_234_567_890); // timestamp
    w.write_i32(0); // isAutoTimestamp
    for v in [0i32, 0, 1280, 720] {
        w.write_i32(v); // crop
    }
    w.write_i32(1); // scaling mode
    w.write_u32(0); // transform
    w.write_u32(0); // sticky transform
    w.write_i32(0); // async
    w.write_i32(1); // swap interval
    w.write_bytes_inplace(&[0u8; 0x24]); // fence
    w.finish()
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("surfaceflinger_queue");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    // Triple-buffered queue. Precondition: a cycle leaves every slot free again.
    let q = SurfaceFlingerQueue::new(3);
    let mut frame = run_frames(&q, 1, FRAMES);
    assert_eq!(q.get_counts(), (3, 0, 0, 0));

    group.throughput(Throughput::Elements(FRAMES));
    group.bench_function(BenchmarkId::new("frame_cycle", "3_buffers_x256"), |b| {
        b.iter(|| {
            frame = run_frames(black_box(&q), frame, FRAMES);
            black_box(frame)
        })
    });

    // 64 free slots with scrambled frame numbers: worst case for the oldest-slot scan.
    let wide = SurfaceFlingerQueue::new(64);
    let mut rng = XorShift64(0x9e37_79b9_7f4a_7c15);
    for slot in 0..64 {
        wide.sync_slot_state(slot, 0, rng.next() >> 16);
    }
    let oldest = wide.find_oldest_free_slot(0).expect("free slot");
    let min_frame = (0..64)
        .map(|s| wide.frame_numbers[s].load(std::sync::atomic::Ordering::Relaxed))
        .min()
        .unwrap();
    assert_eq!(
        wide.frame_numbers[oldest].load(std::sync::atomic::Ordering::Relaxed),
        min_frame
    );

    group.throughput(Throughput::Elements(64));
    group.bench_function(
        BenchmarkId::new("find_oldest_free_slot", "64_free_slots"),
        |b| b.iter(|| black_box(black_box(&wide).find_oldest_free_slot(black_box(0)))),
    );

    let mut token_utf16: Vec<u8> = PRODUCER_TOKEN
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    token_utf16.extend_from_slice(&[0, 0]);
    let mut parcel_buf = vec![0u8; 512];
    let parcel_len = build_queue_buffer_parcel(&mut parcel_buf, &token_utf16);
    let parcel = parcel_buf[..parcel_len].to_vec();
    {
        let mut r = ParcelReader::new(&parcel).expect("valid parcel");
        assert_eq!(r.read_interface_token().as_deref(), Some(PRODUCER_TOKEN));
        assert_eq!(r.read_i32(), Some(1));
    }

    group.throughput(Throughput::Bytes(parcel_len as u64));
    group.bench_function(
        BenchmarkId::new("parcel_write", "queue_buffer_input"),
        |b| {
            b.iter(|| {
                black_box(build_queue_buffer_parcel(
                    black_box(&mut parcel_buf),
                    black_box(&token_utf16),
                ))
            })
        },
    );

    group.bench_function(
        BenchmarkId::new("parcel_read", "interface_token_and_input"),
        |b| {
            b.iter(|| {
                let mut r = ParcelReader::new(black_box(&parcel)).expect("valid parcel");
                let token = r.read_interface_token();
                let slot = r.read_i32();
                let _flat = r.read_i32();
                let ts = r.read_i64();
                black_box((token, slot, ts))
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
