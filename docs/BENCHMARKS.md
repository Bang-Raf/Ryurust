# Benchmark Results

Every number on this page was measured with the Criterion suite in
[`benches/`](../benches). Nothing is estimated or copied from elsewhere. The
raw Criterion output for both runs is in
[`benchmark-results/2026-11-10/`](benchmark-results/2026-11-10).

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
| Load | Laptop in normal use: a web browser was using about one core during both runs |

Settings per benchmark: 30 samples, 0.5 s warm-up, 2 s measurement. Times are
Criterion's point estimate (median of the bootstrap). Two complete runs of the
whole suite were made back to back.

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
- `texture_astc/decode_*` uses pseudo-random blocks the decoder accepts. Blocks
  that make it panic were filtered out (see the ASTC crash in
  `KNOWN_ISSUES.md`). `decode_seq_void_extent` is the constant-colour fast
  path.
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
| `mix_ramp/240` | 540.92 ns | 560.19 ns | +3.6% | 443.69 Melem/s |
| `biquad_df2/240` | 566.81 ns | 578.28 ns | +2.0% | 423.43 Melem/s |
| `biquad_and_mix_df1/240` | 1.4497 µs | 1.3164 µs | -9.2% | 165.55 Melem/s |
| `command_batch/61cmd_64x240` | 24.691 µs | 22.694 µs | -8.1% | 592.92 Melem/s |

### `ryu-audio-fx`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `biquad_filter/240` | 566.14 ns | 585.65 ns | +3.4% | 423.93 Melem/s |
| `biquad_filter_and_mix/240` | 1.2995 µs | 1.3124 µs | +1.0% | 184.69 Melem/s |
| `double_biquad_filter_and_mix/240` | 1.3704 µs | 1.4040 µs | +2.5% | 175.14 Melem/s |
| `multi_biquad_4stage/240` | 2.9588 µs | 2.9848 µs | +0.9% | 81.114 Melem/s |

### `ryu-audio-mixer`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `mix_buffer/240` | 28.354 ns | 24.685 ns | -12.9% ⚠ | 8.4643 Gelem/s |
| `mix_ramp/240` | 572.62 ns | 580.80 ns | +1.4% | 419.12 Melem/s |
| `mix_batch/128voices_x240` | 2.9306 µs | 2.5149 µs | -14.2% ⚠ | 10.482 Gelem/s |
| `mix_matrix/1to6_x240` | 104.87 ns | 116.54 ns | +11.1% ⚠ | 13.731 Gelem/s |

### `ryu-audio-resampler`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `resample_default_4tap/32k_to_48k_240` | 2.2055 µs | 2.1980 µs | -0.3% | 108.82 Melem/s |
| `resample_high_8tap/32k_to_48k_240` | 2.8182 µs | 2.4332 µs | -13.7% ⚠ | 85.162 Melem/s |
| `decode_adpcm/4200` | 10.355 µs | 9.8863 µs | -4.5% | 405.60 Melem/s |

### `ryu-buffer-diff`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `find_dirty_range/4096` | 34.512 ns | 34.458 ns | -0.2% | 110.53 GiB/s |
| `find_dirty_range/65536` | 1.0223 µs | 1.0092 µs | -1.3% | 59.702 GiB/s |
| `is_equal_fast/4096` | 44.740 ns | 49.314 ns | +10.2% ⚠ | 85.263 GiB/s |
| `diff_and_copy/65536` | 1.1015 µs | 1.3362 µs | +21.3% ⚠ | 55.409 GiB/s |

### `ryu-common-hash`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `xxh3_128/4KiB` | 58.424 ns | 59.047 ns | +1.1% | 65.294 GiB/s |
| `xxh3_128/1MiB` | 17.561 µs | 17.616 µs | +0.3% | 55.610 GiB/s |
| `batch_parallel/256x4KiB` | 18.522 µs | 19.342 µs | +4.4% | 52.725 GiB/s |

### `ryu-gal-queue`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `spsc_fill_drain/256` | 7.7705 µs | 7.8724 µs | +1.3% | 32.945 Melem/s |
| `spsc_two_thread_transfer` | 82.771 ns | 83.319 ns | +0.7% | 12.082 Melem/s |
| `span_pool_insert_consume/64` | 22.276 ns | 16.675 ns | -25.1% ⚠ | 2.6757 GiB/s |
| `span_pool_insert_consume/4096` | 103.88 ns | 103.80 ns | -0.1% | 36.721 GiB/s |

### `ryu-gpu-gpfifo`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `parse_pushbuffer_chunk/53_methods` | 54.448 ns | 58.816 ns | +8.0% | 973.41 Melem/s |
| `parse_pushbuffer_chunk/49979_methods` | 44.933 µs | 44.172 µs | -1.7% | 1.1123 Gelem/s |

### `ryu-host1x-syncpoint`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `increment_drain/64` | 981.00 ns | 915.98 ns | -6.6% | 65.240 Melem/s |
| `frame_out_of_order/32` | 972.00 ns | 953.09 ns | -1.9% | 32.922 Melem/s |
| `frame_out_of_order/128` | 5.7400 µs | 5.7043 µs | -0.6% | 22.299 Melem/s |

### `ryu-interval-tree`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `find_overlaps/128` | 1.7116 µs | 1.5706 µs | -8.2% | 37.392 Melem/s |
| `find_overlaps_slice/128` | 1.5669 µs | 1.4398 µs | -8.1% | 40.846 Melem/s |
| `find_overlaps/1024` | 9.0180 µs | 8.9829 µs | -0.4% | 7.0969 Melem/s |
| `find_overlaps_slice/1024` | 8.6399 µs | 8.6073 µs | -0.4% | 7.4075 Melem/s |
| `insert_shuffled/1024` | 55.701 µs | 54.229 µs | -2.6% | 18.384 Melem/s |

### `ryu-ipc-parser`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `parse/cmif_request_128B` | 72.769 ns | 63.993 ns | -12.1% ⚠ | 1.6382 GiB/s |
| `parse/tipc_request_48B` | 68.208 ns | 55.216 ns | -19.0% ⚠ | 671.13 MiB/s |
| `serialize_cmif/response_24B_payload` | 9.6809 ns | 8.7307 ns | -9.8% | 6.1569 GiB/s |
| `serialize_tipc/response_16B_payload` | 5.8768 ns | 6.0368 ns | +2.7% | 3.8034 GiB/s |

### `ryu-jit-regalloc`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `linear_scan/512_sorted` | 5.2686 µs | 5.2723 µs | +0.1% | 97.179 Melem/s |
| `linear_scan/2048_sorted` | 22.351 µs | 21.989 µs | -1.6% | 91.629 Melem/s |
| `linear_scan/512_unsorted` | 11.266 µs | 10.088 µs | -10.5% ⚠ | 45.447 Melem/s |
| `bitset_or/4096bits` | 32.645 ns | 29.888 ns | -8.4% | 14.607 GiB/s |

### `ryu-kernel-scheduler`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `schedule_unschedule/96_threads` | 401.79 ns | 366.89 ns | -8.7% | 238.93 Melem/s |
| `reschedule/6_threads_same_prio` | 5.4200 µs | 5.4870 µs | +1.2% | 188.93 Melem/s |
| `scheduled_element_at/walk_96_threads` | 360.93 ns | 355.02 ns | -1.6% | 265.98 Melem/s |
| `transfer_to_core/32_threads_ping_pong` | 496.75 ns | 496.42 ns | -0.1% | 128.84 Melem/s |

### `ryu-kernel-sync`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `insert_then_wake_all/16addr_x4` | 1.6062 µs | 1.3802 µs | -14.1% ⚠ | 39.845 Melem/s |
| `insert_then_remove/16addr_x4` | 2.0491 µs | 2.0596 µs | +0.5% | 31.233 Melem/s |
| `compute_signal_modify_addend/256addr_populated` | 11.031 µs | 11.114 µs | +0.8% | 92.833 Melem/s |

### `ryu-maxwell-state`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `scan_changes/14KiB` | 131.11 ns | 131.84 ns | +0.6% | 101.83 GiB/s |
| `scan_changes_scalar/14KiB` | 1.1156 µs | 1.0881 µs | -2.5% | 11.968 GiB/s |
| `set_dirty_batch/1024` | 472.30 ns | 394.08 ns | -16.6% ⚠ | 2.1681 Gelem/s |
| `pipeline_state_hash64/72` | 16.425 ns | 15.728 ns | -4.2% | 4.0826 GiB/s |
| `pipeline_state_hash64/1024` | 203.81 ns | 198.47 ns | -2.6% | 4.6793 GiB/s |

### `ryu-memory-pagetable`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `read/random_256MiB` | 4.2913 µs | 4.3969 µs | +2.5% | 954.50 Melem/s |
| `read_mut/random_256MiB` | 4.9074 µs | 4.7477 µs | -3.3% | 834.65 Melem/s |
| `read/sequential_16MiB` | 3.2753 µs | 3.2871 µs | +0.4% | 1.2506 Gelem/s |
| `map_range/16MiB_fresh_table` | 14.464 µs | 12.571 µs | -13.1% ⚠ | 283.19 Melem/s |

### `ryu-memory-tracking`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `is_set/random_1M_bits` | 2.5879 µs | 2.5426 µs | -1.8% | 1.5828 Gelem/s |
| `set_range/2MiB_set_then_clear_x1024` | 24.558 µs | 22.826 µs | -7.1% | 42.698 Gelem/s |
| `find_next_set/scan_1pct_dirty` | 57.837 µs | 63.822 µs | +10.3% ⚠ | 180.27 Melem/s |
| `count_set_bits/1M_bits` | 12.650 µs | 12.511 µs | -1.1% | 82.894 Gelem/s |

### `ryu-motion-fusion`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `update/1000samples` | 23.253 µs | 22.408 µs | -3.6% | 43.004 Melem/s |
| `update_and_matrix/1000samples` | 26.531 µs | 24.304 µs | -8.4% | 37.692 Melem/s |
| `rotation_matrix_3x3` | 2.9203 ns | 2.8597 ns | -2.1% | 342.43 Melem/s |

### `ryu-pixel-converter`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `r5g6b5_to_rgba8/1920x1080` | 518.35 µs | 506.08 µs | -2.4% | 4.0004 Gelem/s |
| `r4g4b4a4_to_rgba8/1920x1080` | 420.33 µs | 411.13 µs | -2.2% | 4.9332 Gelem/s |
| `bgra8_to_rgba8/1920x1080` | 420.15 µs | 404.59 µs | -3.7% | 4.9354 Gelem/s |

### `ryu-romfs-indexer`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `find_file/hit_random_10k_files` | 160.27 µs | 161.03 µs | +0.5% | 6.3890 Melem/s |
| `find_file/miss_random_10k_files` | 103.76 µs | 73.657 µs | -29.0% ⚠ | 9.8691 Melem/s |
| `reader_find_file/hit_random_10k_files` | 156.77 µs | 137.14 µs | -12.5% ⚠ | 6.5317 Melem/s |
| `from_metadata_bytes/10k_files` | 39.308 µs | 39.276 µs | -0.1% | 14.738 GiB/s |

### `ryu-shader-cache`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `lz4_decompress/64KiB_from_15735B` | 35.746 µs | 37.749 µs | +5.6% | 1.7075 GiB/s |
| `crc32/64KiB` | 120.91 µs | 102.58 µs | -15.2% ⚠ | 516.90 MiB/s |
| `toc_parse/4096` | 1.8122 µs | 1.8030 µs | -0.5% | 2.2603 Gelem/s |

### `ryu-shader-transpiler`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `transpile/4KiB` | 1.4041 µs | 1.4024 µs | -0.1% | 2.7168 GiB/s |
| `transpile/64KiB` | 27.451 µs | 26.984 µs | -1.7% | 2.2234 GiB/s |
| `decode_instructions/64KiB` | 15.932 µs | 15.574 µs | -2.2% | 3.8309 GiB/s |

### `ryu-storage-fs`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `aes_ctr_decrypt/16KiB` | 6.3293 µs | 6.4479 µs | +1.9% | 2.4108 GiB/s |
| `aes_ctr_decrypt/1024KiB` | 404.67 µs | 406.35 µs | +0.4% | 2.4133 GiB/s |
| `lz4_decompress/64KiB` | 9.6222 µs | 9.5610 µs | -0.6% | 6.3432 GiB/s |
| `zstd_decompress/64KiB_level3` | 6.1527 µs | 6.0554 µs | -1.6% | 9.9201 GiB/s |
| `pfs0_find_file/all_64_names` | 35.530 µs | 34.975 µs | -1.6% | 1.8013 Melem/s |

### `ryu-surfaceflinger-queue`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `frame_cycle/3_buffers_x256` | 12.257 µs | 11.778 µs | -3.9% | 20.886 Melem/s |
| `find_oldest_free_slot/64_free_slots` | 47.852 ns | 46.836 ns | -2.1% | 1.3375 Gelem/s |
| `parcel_write/queue_buffer_input` | 9.8634 ns | 9.5270 ns | -3.4% | 17.751 GiB/s |
| `parcel_read/interface_token_and_input` | 42.862 ns | 43.252 ns | +0.9% | 4.0850 GiB/s |

### `ryu-tegra-mme`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `execute_alu_seq` | 22.946 ns | 23.664 ns | +3.1% | 348.64 Melem/s |
| `execute_indirect_draw/10` | 199.11 ns | 203.10 ns | +2.0% | 50.225 Melem/s |
| `execute_indirect_draw/200` | 3.6144 µs | 3.7279 µs | +3.1% | 55.334 Melem/s |
| `execute_indirect_draw/2000` | 36.034 µs | 36.803 µs | +2.1% | 55.503 Melem/s |

### `ryu-tegra-swizzle`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `deswizzle_seq/rgba8_1920x1080` | 519.57 µs | 536.71 µs | +3.3% | 14.868 GiB/s |
| `deswizzle_par/rgba8_1920x1080` | 161.42 µs | 180.10 µs | +11.6% ⚠ | 47.855 GiB/s |
| `swizzle_seq/rgba8_1920x1080` | 633.11 µs | 636.79 µs | +0.6% | 12.201 GiB/s |
| `swizzle_par/rgba8_1920x1080` | 179.40 µs | 179.67 µs | +0.2% | 43.059 GiB/s |

### `ryu-texture-astc`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `decode_seq/4x4_1920x1080` | 65.959 ms | 65.394 ms | -0.9% | 119.93 MiB/s |
| `decode_par/4x4_1920x1080` | 13.282 ms | 13.732 ms | +3.4% | 595.54 MiB/s |
| `decode_seq/8x8_1920x1080` | 36.179 ms | 36.667 ms | +1.3% | 218.64 MiB/s |
| `decode_par/8x8_1920x1080` | 7.7303 ms | 7.3562 ms | -4.8% | 1023.3 MiB/s |
| `decode_seq_void_extent/4x4_1920x1080` | 2.5356 ms | 2.5463 ms | +0.4% | 3.0465 GiB/s |

### `ryu-texture-bcn`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `bc1/1920x1080` | 1.6663 ms | 1.6454 ms | -1.3% | 4.6360 GiB/s |
| `bc3/1920x1080` | 2.2721 ms | 2.2320 ms | -1.8% | 3.3998 GiB/s |
| `bc6h_ufloat/1920x1080` | 7.1732 ms | 7.0206 ms | -2.1% | 2.1538 GiB/s |
| `bc7/1920x1080` | 13.223 ms | 12.767 ms | -3.4% | 598.22 MiB/s |

### `ryu-texture-matcher`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `view_format_compatible/4096` | 5.7813 µs | 5.4391 µs | -5.9% | 708.50 Melem/s |
| `view_target_compatible/4096` | 16.311 µs | 16.028 µs | -1.7% | 251.11 Melem/s |
| `full_match_query/4096` | 1.2735 µs | 1.2806 µs | +0.6% | 3.2163 Gelem/s |

### `ryu-video-nvdec`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `idct8x8_full_add/64px` | 121.98 ns | 122.95 ns | +0.8% | 524.69 Melem/s |
| `idct32x32_full_add/1024px` | 2.8012 µs | 2.7844 µs | -0.6% | 365.56 Melem/s |
| `loop_filter_frame/1920x1080_i420` | 9.4205 ms | 9.2991 ms | -1.3% | 314.88 MiB/s |
| `synthetic_dc_reconstruct_frame/1920x1080_i420` | 5.5367 ms | 5.5448 ms | +0.1% | 535.75 MiB/s |

### `ryu-video-vic`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `blend_surface_bt709/1920x1080_rgba16` | 3.9886 ms | 3.6796 ms | -7.7% | 3.8734 GiB/s |
| `blend_surface_passthrough/1920x1080_rgba16` | 767.40 µs | 705.75 µs | -8.0% | 20.132 GiB/s |
| `deinterlace_motion_adaptive/1920x1080_luma` | 209.70 µs | 206.59 µs | -1.5% | 9.2092 GiB/s |
| `deinterlace_bob/1920x1080_luma` | 22.406 µs | 22.063 µs | -1.5% | 86.193 GiB/s |

### `ryu-vulkan-descriptors`

| Benchmark | Run 1 | Run 2 | Δ | Throughput (run 1) |
| :--- | ---: | ---: | ---: | ---: |
| `update_buffer/90` | 121.33 ns | 118.96 ns | -2.0% | 741.77 Melem/s |
| `update_filter_buffers_per_draw/100` | 805.37 ns | 806.88 ns | +0.2% | 124.17 Melem/s |
| `update_filter_images_per_draw/100` | 4.3830 µs | 4.4999 µs | +2.7% | 22.815 Melem/s |
| `pack_template_buffer_slice/18` | 3.2874 ns | 4.2154 ns | +28.2% ⚠ | 122.39 GiB/s |
