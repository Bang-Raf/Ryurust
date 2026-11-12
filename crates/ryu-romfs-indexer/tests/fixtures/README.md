# RomFS test fixtures

`libhac_synthetic.romfs` is a RomFS image written by the reference implementation,
LibHac, and read by `tests/libhac_image.rs` through this crate's public API.

- Built with NuGet package `Ryujinx.LibHac` `0.21.0-alpha.133` (the version Ryubing
  pins in `Directory.Packages.props`), class
  `LibHac.Tools.FsSystem.RomFs.RomFsBuilder`, on .NET 10.
- All content is synthetic. File names are made up by the generator and file bytes come
  from a xorshift generator seeded by the path. Nothing comes from a game, firmware or
  key file.
- 36,040 bytes; 314 files and 80 directories plus the root. It covers nesting nine levels
  deep, three empty directories, empty files, 220 files in one directory (the file hash
  table has 317 buckets, so chains collide), names over 200 bytes, names that differ
  only by case, non-ASCII UTF-8 names, and a path where a file and its parent
  directories share one name.
- Empty directories: `RomFsBuilder` only has `AddFile`, so the generator calls LibHac's
  public `HierarchicalRomFileTable.AddDirectory` on the builder's own table, which it
  reaches by reflection.

`libhac_synthetic.manifest.tsv` was written by opening the built image with LibHac's
`RomFsFileSystem` and listing it recursively. Lines are tab-separated:

    D <path>
    F <path> <absolute data offset in the image> <size> <FNV-1a 64 of the contents, hex>

The offset is `header.DataOffset + RomFileInfo.Offset` from
`RomFsFileSystem.FileTable.TryOpenFile`; the generator checks that the bytes at that
offset equal what `RomFsFileSystem.OpenFile` reads.

## Regenerating

`generator.cs.txt` is the generator's `Program.cs`. Put it in a console project that
targets `net10.0` and has
`<PackageReference Include="Ryujinx.LibHac" Version="0.21.0-alpha.133" />`, then run
`dotnet run -c Release -- <output directory>`. Output is deterministic: two runs give
byte-identical files. If the tree changes, update `MANIFEST_DIRS` and `MANIFEST_FILES`
in the test.
