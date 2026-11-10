# Benchmark Results

Every number on this page was measured with the Criterion suite in
[`benches/`](../benches). Nothing is estimated or copied from elsewhere. The
raw Criterion output for both runs is in
[`benchmark-results/2026-11-10/`](benchmark-results/2026-11-10). The results
are from the code as released, after the fixes listed in
[`KNOWN_ISSUES.md`](KNOWN_ISSUES.md). Output from the earlier pre-fix runs is
kept in `benchmark-results/2026-11-10/old/` for reference.

## What these numbers are (and aren't)

- They measure **the Rust crates on their own**, through their safe Rust API.
  They don't include P/Invoke overhead or the rest of the emulator.
- **There is no C# comparison.** Nothing here tells you how much faster or
  slower a crate is than the Ryujinx code it replaces. That needs the C# path
  measured on the same input in the same process, which isn't built yet. Don't
  quote these numbers as speedups.
- Inputs are synthetic but realistic in size (1920×1080 surfaces, 240-sample
  audio frames at 48 kHz, 4 KiB pages, 64 KiB shaders). Small inputs fit in
  L1/L2 cache, so throughput can be far above memory bandwidth.
- Some crates do less than their names say (see
  [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md)). In particular, `video_nvdec` isn't a VP9
  decoder and `shader_transpiler` doesn't produce valid SPIR-V, so their
  numbers time incomplete implementations.
- Benchmarks whose two runs differ by more than 10% are marked ⚠. Treat those
  as rough.

## Test machine

| | |
| :--- | :--- |
| CPU | Intel Core Ultra 7 256V (Lunar Lake), 8 cores / 8 threads, up to 4.7 GHz, AVX2/FMA, no AVX-512 |
| Memory | 16 GB LPDDR5X (15 GiB visible) |
| OS | Fedora Linux 44, kernel 7.2.13-200.fc44.x86_64 |
| Power | AC, `performance` governor |
| Toolchain | rustc 1.99.1, criterion 0.5.1, `bench` profile (`opt-level=3`, thin LTO, 1 codegen unit), no `target-cpu` flags |
| Date | 2026-11-10 |
| Load | Laptop in normal use (web browser open, load average 5–10 during the runs) |

Settings per benchmark: 30 samples, 0.5 s warm-up, 2 s measurement. Times are
Criterion's point estimate (median of the bootstrap). Two complete runs of the
whole suite were made back to back.

## Effect of the pre-release fixes

Benchmarks that changed by more than 15% between the pre-fix and released
code, same machine and settings:

| Benchmark | Before | After | Why |
| :--- | ---: | ---: | :--- |
| `shader_cache/crc32/64KiB` | 121 µs | 29.5 µs | slicing-by-8 CRC32 |
| `shader_cache/lz4_decompress` | 35.7 µs | 21.3 µs | block copies for matches |
| `romfs_indexer/find_file/miss_*` | 104 µs | 54.4 µs | table layout cached |
| `video_vic/blend_surface_bt709` | 3.99 ms | 2.60 ms | parallel path rewritten with `par_chunks_mut` |
| `jit_regalloc/linear_scan/512_sorted` | 5.27 µs | 6.58 µs | fixed-interval lookahead (correctness fix) |
| `host1x_syncpoint/frame_out_of_order/32` | 972 ns | 1.41 µs | hash-map index; slower for small queues, O(1) for large |
| `ipc_parser/serialize_tipc` | 5.9 ns | 8.2 ns | raw-data padding now written |
| `texture_astc/*` | — | — | not comparable: the input mix changed |

The first alignment-safe rewrite of `ryu-pixel-converter` was 2–2.5× slower;
it was rewritten with `as_chunks` and now matches the pre-fix speed.

## Reproduce

```bash
cargo bench -p ryurust-bench                     # full suite, several minutes
cargo bench -p ryurust-bench --bench texture_bcn # one crate
```

To compare your change against `main`:

```bash
git checkout main && cargo bench -p ryurust-bench --bench texture_bcn -- --save-baseline main
git checkout -    && cargo bench -p ryurust-bench --bench texture_bcn -- --baseline main
```

When you publish new results, add a dated folder under `benchmark-results/`
with the raw output and update this page in the same PR.

## Notes on specific benchmarks

- `*_par` / `batch_parallel` / `video_vic/blend_surface_*` use Rayon across all
  8 cores; everything else is single-threaded.
- `texture_astc/decode_*` uses pseudo-random non-void-extent blocks. Many of
  them are invalid encodings and take the fast error-colour path, so these
  numbers are optimistic for real game textures. `decode_seq_void_extent` is
  the constant-colour fast path.
- `texture_matcher/full_match_query` short-circuits: most pairs fail the first
  check, which is why it's faster than its parts.
- `romfs_indexer/find_file` vs `reader_find_file` shows the cost of rebuilding
  the reader on every lookup.
- `memory_pagetable/read/random_*` uses seeded random addresses across 256 MiB
  of mapped memory; `sequential_*` is the cache-friendly case for comparison.
- `video_vic/deinterlace_*` report throughput against the full frame size,
  though only half the lines are written.

## Results (2026-11-10)

### `ryu-audio-dsp`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `mix_ramp/240` | 543.93 ns | 545.04 ns | +0.2% | 441.24 Melem/s |
| `biquad_df2/240` | 544.47 ns | 555.33 ns | +2.0% | 440.79 Melem/s |
| `biquad_and_mix_df1/240` | 1.3221 µs | 1.3168 µs | -0.4% | 181.53 Melem/s |
| `command_batch/61cmd_64x240` | 23.495 µs | 23.253 µs | -1.0% | 623.12 Melem/s |

### `ryu-audio-fx`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `biquad_filter/240` | 574.75 ns | 576.83 ns | +0.4% | 417.57 Melem/s |
| `biquad_filter_and_mix/240` | 1.3318 µs | 1.3024 µs | -2.2% | 180.21 Melem/s |
| `double_biquad_filter_and_mix/240` | 1.4423 µs | 1.3821 µs | -4.2% | 166.40 Melem/s |
| `multi_biquad_4stage/240` | 2.9949 µs | 2.9909 µs | -0.1% | 80.136 Melem/s |

### `ryu-audio-mixer`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `mix_buffer/240` | 23.562 ns | 24.629 ns | +4.5% | 10.186 Gelem/s |
| `mix_ramp/240` | 546.36 ns | 533.21 ns | -2.4% | 439.27 Melem/s |
| `mix_batch/128voices_x240` | 2.4672 µs | 2.4531 µs | -0.6% | 12.451 Gelem/s |
| `mix_matrix/1to6_x240` | 91.696 ns | 90.316 ns | -1.5% | 15.704 Gelem/s |

### `ryu-audio-resampler`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `resample_default_4tap/32k_to_48k_240` | 2.2859 µs | 2.2101 µs | -3.3% | 104.99 Melem/s |
| `resample_high_8tap/32k_to_48k_240` | 2.4800 µs | 2.4251 µs | -2.2% | 96.772 Melem/s |
| `decode_adpcm/4200` | 10.141 µs | 9.8741 µs | -2.6% | 414.17 Melem/s |

### `ryu-buffer-diff`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `find_dirty_range/4096` | 35.050 ns | 34.410 ns | -1.8% | 108.84 GiB/s |
| `find_dirty_range/65536` | 1.0778 µs | 1.0249 µs | -4.9% | 56.628 GiB/s |
| `is_equal_fast/4096` | 45.291 ns | 51.683 ns | +14.1% ⚠ | 84.226 GiB/s |
| `diff_and_copy/65536` | 1.0820 µs | 1.0988 µs | +1.6% | 56.412 GiB/s |

### `ryu-common-hash`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `xxh3_128/4KiB` | 59.489 ns | 60.126 ns | +1.1% | 64.124 GiB/s |
| `xxh3_128/1MiB` | 17.930 µs | 17.572 µs | -2.0% | 54.465 GiB/s |
| `batch_parallel/256x4KiB` | 19.469 µs | 21.405 µs | +9.9% | 50.160 GiB/s |

### `ryu-gal-queue`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `spsc_fill_drain/256` | 7.9397 µs | 7.7873 µs | -1.9% | 32.243 Melem/s |
| `spsc_two_thread_transfer` | 84.241 ns | 83.171 ns | -1.3% | 11.871 Melem/s |
| `span_pool_insert_consume/64` | 16.965 ns | 16.803 ns | -1.0% | 3.5133 GiB/s |
| `span_pool_insert_consume/4096` | 103.98 ns | 104.40 ns | +0.4% | 36.686 GiB/s |

### `ryu-gpu-gpfifo`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `parse_pushbuffer_chunk/53_methods` | 55.097 ns | 56.006 ns | +1.6% | 961.93 Melem/s |
| `parse_pushbuffer_chunk/49979_methods` | 44.516 µs | 45.328 µs | +1.8% | 1.1227 Gelem/s |

### `ryu-host1x-syncpoint`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `increment_drain/64` | 1.0156 µs | 1.0486 µs | +3.2% | 63.016 Melem/s |
| `frame_out_of_order/32` | 1.4075 µs | 1.4463 µs | +2.8% | 22.736 Melem/s |
| `frame_out_of_order/128` | 5.7855 µs | 5.8343 µs | +0.8% | 22.124 Melem/s |

### `ryu-interval-tree`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `find_overlaps/128` | 1.5926 µs | 1.6366 µs | +2.8% | 40.185 Melem/s |
| `find_overlaps_slice/128` | 1.4330 µs | 1.4317 µs | -0.1% | 44.663 Melem/s |
| `find_overlaps/1024` | 8.8951 µs | 8.8648 µs | -0.3% | 7.1949 Melem/s |
| `find_overlaps_slice/1024` | 8.5587 µs | 8.6858 µs | +1.5% | 7.4777 Melem/s |
| `insert_shuffled/1024` | 54.948 µs | 53.946 µs | -1.8% | 18.636 Melem/s |

### `ryu-ipc-parser`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `parse/cmif_request_128B` | 61.821 ns | 68.922 ns | +11.5% ⚠ | 1.9283 GiB/s |
| `parse/tipc_request_48B` | 53.034 ns | 56.054 ns | +5.7% | 863.15 MiB/s |
| `serialize_cmif/response_24B_payload` | 10.870 ns | 11.097 ns | +2.1% | 5.4834 GiB/s |
| `serialize_tipc/response_16B_payload` | 8.2018 ns | 8.3164 ns | +1.4% | 2.7252 GiB/s |

### `ryu-jit-regalloc`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `linear_scan/512_sorted` | 6.5758 µs | 6.5429 µs | -0.5% | 77.861 Melem/s |
| `linear_scan/2048_sorted` | 27.942 µs | 27.412 µs | -1.9% | 73.295 Melem/s |
| `linear_scan/512_unsorted` | 12.383 µs | 11.592 µs | -6.4% | 41.346 Melem/s |
| `bitset_or/4096bits` | 29.073 ns | 28.370 ns | -2.4% | 16.401 GiB/s |

### `ryu-kernel-scheduler`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `schedule_unschedule/96_threads` | 355.24 ns | 364.41 ns | +2.6% | 270.24 Melem/s |
| `reschedule/6_threads_same_prio` | 5.3735 µs | 5.2596 µs | -2.1% | 190.56 Melem/s |
| `scheduled_element_at/walk_96_threads` | 357.14 ns | 349.70 ns | -2.1% | 268.80 Melem/s |
| `transfer_to_core/32_threads_ping_pong` | 492.93 ns | 495.47 ns | +0.5% | 129.83 Melem/s |

### `ryu-kernel-sync`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `insert_then_wake_all/16addr_x4` | 1.3955 µs | 1.3665 µs | -2.1% | 45.862 Melem/s |
| `insert_then_remove/16addr_x4` | 2.0058 µs | 1.9958 µs | -0.5% | 31.907 Melem/s |
| `compute_signal_modify_addend/256addr_populated` | 11.178 µs | 10.976 µs | -1.8% | 91.611 Melem/s |

### `ryu-maxwell-state`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `scan_changes/14KiB` | 132.17 ns | 131.48 ns | -0.5% | 101.02 GiB/s |
| `scan_changes_scalar/14KiB` | 1.1410 µs | 1.0716 µs | -6.1% | 11.702 GiB/s |
| `set_dirty_batch/1024` | 413.51 ns | 396.04 ns | -4.2% | 2.4763 Gelem/s |
| `pipeline_state_hash64/72` | 16.346 ns | 15.925 ns | -2.6% | 4.1023 GiB/s |
| `pipeline_state_hash64/1024` | 205.74 ns | 197.20 ns | -4.2% | 4.6353 GiB/s |

### `ryu-memory-pagetable`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `read/random_256MiB` | 4.2377 µs | 4.1946 µs | -1.0% | 966.57 Melem/s |
| `read_mut/random_256MiB` | 4.4390 µs | 4.7185 µs | +6.3% | 922.73 Melem/s |
| `read/sequential_16MiB` | 3.2412 µs | 3.2603 µs | +0.6% | 1.2637 Gelem/s |
| `map_range/16MiB_fresh_table` | 13.281 µs | 12.626 µs | -4.9% | 308.42 Melem/s |

### `ryu-memory-tracking`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `is_set/random_1M_bits` | 2.5637 µs | 2.5500 µs | -0.5% | 1.5977 Gelem/s |
| `set_range/2MiB_set_then_clear_x1024` | 22.784 µs | 22.689 µs | -0.4% | 46.023 Gelem/s |
| `find_next_set/scan_1pct_dirty` | 65.449 µs | 65.099 µs | -0.5% | 159.30 Melem/s |
| `count_set_bits/1M_bits` | 12.530 µs | 12.012 µs | -4.1% | 83.683 Gelem/s |

### `ryu-motion-fusion`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `update/1000samples` | 22.428 µs | 21.879 µs | -2.4% | 44.586 Melem/s |
| `update_and_matrix/1000samples` | 24.912 µs | 23.895 µs | -4.1% | 40.141 Melem/s |
| `rotation_matrix_3x3` | 2.9137 ns | 2.8568 ns | -2.0% | 343.21 Melem/s |

### `ryu-romfs-indexer`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `find_file/hit_random_10k_files` | 144.87 µs | 146.85 µs | +1.4% | 7.0683 Melem/s |
| `find_file/miss_random_10k_files` | 54.407 µs | 53.930 µs | -0.9% | 18.821 Melem/s |
| `reader_find_file/hit_random_10k_files` | 138.08 µs | 136.19 µs | -1.4% | 7.4162 Melem/s |
| `from_metadata_bytes/10k_files` | 39.671 µs | 40.216 µs | +1.4% | 14.603 GiB/s |

### `ryu-shader-cache`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `lz4_decompress/64KiB_from_15735B` | 21.265 µs | 21.376 µs | +0.5% | 2.8703 GiB/s |
| `crc32/64KiB` | 29.482 µs | 29.555 µs | +0.2% | 2.0702 GiB/s |
| `toc_parse/4096` | 1.7822 µs | 1.7781 µs | -0.2% | 2.2983 Gelem/s |

### `ryu-shader-transpiler`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `transpile/4KiB` | 1.4157 µs | 1.3859 µs | -2.1% | 2.6946 GiB/s |
| `transpile/64KiB` | 27.040 µs | 27.565 µs | +1.9% | 2.2572 GiB/s |
| `decode_instructions/64KiB` | 16.276 µs | 16.135 µs | -0.9% | 3.7500 GiB/s |

### `ryu-storage-fs`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `aes_ctr_decrypt/16KiB` | 6.7873 µs | 6.6288 µs | -2.3% | 2.2481 GiB/s |
| `aes_ctr_decrypt/1024KiB` | 421.77 µs | 412.57 µs | -2.2% | 2.3154 GiB/s |
| `lz4_decompress/64KiB` | 9.8942 µs | 9.6952 µs | -2.0% | 6.1688 GiB/s |
| `zstd_decompress/64KiB_level3` | 5.3390 µs | 5.2188 µs | -2.3% | 11.432 GiB/s |
| `pfs0_find_file/all_64_names` | 35.960 µs | 34.869 µs | -3.0% | 1.7798 Melem/s |

### `ryu-surfaceflinger-queue`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `frame_cycle/3_buffers_x256` | 12.626 µs | 12.146 µs | -3.8% | 20.276 Melem/s |
| `find_oldest_free_slot/64_free_slots` | 47.168 ns | 46.368 ns | -1.7% | 1.3568 Gelem/s |
| `parcel_write/queue_buffer_input` | 10.340 ns | 9.5644 ns | -7.5% | 16.933 GiB/s |
| `parcel_read/interface_token_and_input` | 43.468 ns | 43.778 ns | +0.7% | 4.0280 GiB/s |

### `ryu-tegra-mme`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `execute_alu_seq` | 25.266 ns | 22.934 ns | -9.2% | 316.63 Melem/s |
| `execute_indirect_draw/10` | 217.08 ns | 220.65 ns | +1.6% | 46.067 Melem/s |
| `execute_indirect_draw/200` | 4.0414 µs | 4.1022 µs | +1.5% | 49.488 Melem/s |
| `execute_indirect_draw/2000` | 41.509 µs | 41.623 µs | +0.3% | 48.182 Melem/s |

### `ryu-tegra-swizzle`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `deswizzle_seq/rgba8_1920x1080` | 521.09 µs | 527.04 µs | +1.1% | 14.824 GiB/s |
| `deswizzle_par/rgba8_1920x1080` | 181.05 µs | 179.14 µs | -1.1% | 42.666 GiB/s |
| `swizzle_seq/rgba8_1920x1080` | 641.01 µs | 848.41 µs | +32.4% ⚠ | 12.051 GiB/s |
| `swizzle_par/rgba8_1920x1080` | 194.31 µs | 233.84 µs | +20.3% ⚠ | 39.755 GiB/s |

### `ryu-texture-astc`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `decode_seq/4x4_1920x1080` | 6.9277 ms | 6.8769 ms | -0.7% | 1.1151 GiB/s |
| `decode_par/4x4_1920x1080` | 1.9015 ms | 1.7319 ms | -8.9% | 4.0624 GiB/s |
| `decode_seq/8x8_1920x1080` | 8.7659 ms | 9.0447 ms | +3.2% | 902.38 MiB/s |
| `decode_par/8x8_1920x1080` | 2.0945 ms | 2.0442 ms | -2.4% | 3.6881 GiB/s |
| `decode_seq_void_extent/4x4_1920x1080` | 2.4812 ms | 2.4591 ms | -0.9% | 3.1134 GiB/s |

### `ryu-texture-bcn`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `bc1/1920x1080` | 1.6881 ms | 1.7022 ms | +0.8% | 4.5761 GiB/s |
| `bc3/1920x1080` | 2.2848 ms | 2.3371 ms | +2.3% | 3.3809 GiB/s |
| `bc6h_ufloat/1920x1080` | 7.3131 ms | 7.2538 ms | -0.8% | 2.1126 GiB/s |
| `bc7/1920x1080` | 13.329 ms | 13.283 ms | -0.3% | 593.47 MiB/s |

### `ryu-texture-matcher`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `view_format_compatible/4096` | 5.7866 µs | 5.7283 µs | -1.0% | 707.85 Melem/s |
| `view_target_compatible/4096` | 16.113 µs | 16.044 µs | -0.4% | 254.20 Melem/s |
| `full_match_query/4096` | 1.2706 µs | 1.2759 µs | +0.4% | 3.2237 Gelem/s |

### `ryu-video-nvdec`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `idct8x8_full_add/64px` | 124.56 ns | 125.67 ns | +0.9% | 513.81 Melem/s |
| `idct32x32_full_add/1024px` | 2.8149 µs | 2.7848 µs | -1.1% | 363.78 Melem/s |
| `loop_filter_frame/1920x1080_i420` | 9.3194 ms | 9.6054 ms | +3.1% | 318.29 MiB/s |
| `synthetic_dc_reconstruct_frame/1920x1080_i420` | 5.6873 ms | 5.6652 ms | -0.4% | 521.57 MiB/s |

### `ryu-video-vic`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `blend_surface_bt709/1920x1080_rgba16` | 2.5975 ms | 2.7199 ms | +4.7% | 5.9479 GiB/s |
| `blend_surface_passthrough/1920x1080_rgba16` | 618.15 µs | 690.43 µs | +11.7% ⚠ | 24.993 GiB/s |
| `deinterlace_motion_adaptive/1920x1080_luma` | 211.69 µs | 208.92 µs | -1.3% | 9.1229 GiB/s |
| `deinterlace_bob/1920x1080_luma` | 22.354 µs | 23.021 µs | +3.0% | 86.390 GiB/s |

### `ryu-vulkan-descriptors`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `update_buffer/90` | 121.68 ns | 122.03 ns | +0.3% | 739.64 Melem/s |
| `update_filter_buffers_per_draw/100` | 740.15 ns | 755.25 ns | +2.0% | 135.11 Melem/s |
| `update_filter_images_per_draw/100` | 4.6686 µs | 4.6571 µs | -0.2% | 21.420 Melem/s |
| `pack_template_buffer_slice/18` | 3.9529 ns | 3.8957 ns | -1.4% | 101.78 GiB/s |

### `ryu-pixel-converter`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `r5g6b5_to_rgba8/1920x1080` | 512.77 µs | 513.27 µs | +0.1% | 4.0439 Gelem/s |
| `r4g4b4a4_to_rgba8/1920x1080` | 422.59 µs | 421.23 µs | -0.3% | 4.9069 Gelem/s |
| `bgra8_to_rgba8/1920x1080` | 403.49 µs | 418.31 µs | +3.7% | 5.1391 Gelem/s |
