# ASTC golden fixtures

Reference output from Ryujinx's C# `AstcDecoder`
(`src/Ryujinx.Graphics.Texture/Astc`, Ryubing tag `Canary-1.3.352`, Release build),
checked by `../golden.rs`.

## How they were generated

A throwaway .NET 10 console program referenced
`Ryujinx.Graphics.Texture.csproj` and, for each fixture:

1. Seeded a splitmix64 PRNG with FNV-1a of the fixture name (base seed
   `0xA57C601D20261111`), so every fixture is deterministic and independent.
2. Filled the texture's blocks in a repeating cycle of ten categories: 1-, 2-, 3- and
   4-partition blocks; dual-plane with 1 partition; dual-plane with 2–3 partitions;
   LDR void-extent; blocks the C# decoder accepts although they break the spec's weight
   limits (more than 64 weights, or weight bits outside 24–96); invalid blocks; and
   unconstrained random blocks. Invalid blocks cycle through: reserved block mode, HDR
   void-extent, void-extent with a cleared reserved bit, weight grid larger than the
   block, 4 partitions with dual plane, and any other block the C# decoder rejects
   (e.g. HDR endpoint modes). Each block is a random 128-bit value with its header bits
   nudged toward the category, kept only when its parsed header and the C# verdict put
   it in that category. A category that cannot occur for a block size (e.g. a grid
   larger than a 12x12 block) falls back to another kind.
3. Skipped blocks that decode more than 100 weight entries while needing fewer than
   128 weight bits. The C# `IntegerSequence` holds 100 entries with no bounds check, so
   its output for those blocks is undefined.
4. Recorded each block's C# `DecompressBlock` verdict (exception = failed), then decoded
   the whole texture with `AstcDecoder.TryDecodeToRgba8` into a buffer pre-filled with
   `0xCD` and saved the returned status and output.

| Fixture | Footprint | Texture | Blocks (failed in C#) |
| :--- | :--- | :--- | :--- |
| `blocks_4x4` | 4x4 | 80x80 | 400 (77) |
| `blocks_5x5` | 5x5 | 80x80 | 256 (50) |
| `blocks_6x5` | 6x5 | 84x70 | 196 (37) |
| `blocks_6x6` | 6x6 | 90x72 | 180 (35) |
| `blocks_8x8` | 8x8 | 80x80 | 100 (19) |
| `blocks_10x8` | 10x8 | 100x64 | 80 (15) |
| `blocks_10x10` | 10x10 | 80x80 | 64 (11) |
| `blocks_12x12` | 12x12 | 96x72 | 48 (7) |
| `layout_4x4_37x29_mips3_layers2` | 4x4 | 37x29, 3 levels, 2 layers | 212 (39) |
| `layout_8x8_20x13x3_mips2` | 8x8 | 20x13x3 (3D), 2 levels | 20 (4) |
| `layout_12x12_7x5_mips3_layers3` | 12x12 | 7x5, 3 levels, 3 layers | 9 (1) |
| `valid_only_6x6_18x12` | 6x6 | 18x12 | 6 (0) |

`valid_only_6x6_18x12` holds only accepted blocks, so the success status is covered too.

## File format (little-endian)

| Offset | Content |
| :--- | :--- |
| 0 | magic `RYUASTC1` |
| 8 | `u32` × 9: block width, block height, width, height, depth, levels, layers, block count *n*, output length *m* |
| 44 | `u8` C# `TryDecodeToRgba8` result (1 = true), then 3 zero bytes |
| 48 | *n* × 16 bytes: ASTC blocks |
| 48 + 16*n* | *n* bytes: C# per-block verdict (1 = decoded, 0 = threw) |
| 48 + 17*n* | *m* bytes: C# RGBA8 output |

## Failed blocks

The C# decoder leaves a failed block's texels zero (its per-block scratch buffer is
zero-initialised and the exception fires before any texel is written). Since v0.1.2 this
crate writes zeros there too (up to v0.1.1 it wrote the ASTC error colour, opaque
magenta), so `golden.rs` requires every output byte, every per-block verdict and the
image-level status to match exactly.

The generator's source is kept next to the fixtures as `generator.cs.txt`
(`Program.cs`) and `generator.csproj.txt` (its project file; point the
`ProjectReference` at your Ryubing checkout). Run `gen <outdir>` to recreate these
fixtures, or `fuzz <outdir> <blocks-per-footprint>` for a larger set, then check a
directory with `ASTC_GOLDEN_DIR=<dir> cargo test -p ryu-texture-astc --test golden -- --ignored`.
