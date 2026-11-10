# Releasing

A release ships two things:

1. The Rust crates at a tagged version (`vX.Y.Z`).
2. A Linux x64 build of Ryubing with those crates enabled, packaged as
   `ryurust-X.Y.Z-ryubing-<ryubing-version>-linux_x64.tar.xz` plus a
   `.sha256` file.

## How the Ryubing build is made

Ryurust doesn't fork the whole Ryubing repository. Instead:

- [`integration/ryubing/BASE`](../integration/ryubing/BASE) pins the upstream
  Ryubing tag and commit (currently `Canary-1.3.352`, `a61a426`).
- [`integration/ryubing/ryurust-integration.patch`](../integration/ryubing/ryurust-integration.patch)
  holds every C# change: the 13 `Native*.cs` wrappers (identical to
  `bindings/csharp/`) and the call sites that use them, each guarded by
  `IsAvailable` with the original C# code as fallback.
- [`scripts/package-linux.sh`](../scripts/package-linux.sh) clones that tag,
  checks the commit, applies the patch, builds the crates and Ryubing, and
  packs the result.

The Ryubing build is made the same way as upstream's release workflow
(`dotnet publish -c Release -r linux-x64 --self-contained -p:DebugType=embedded`),
with two differences:

- `-p:Version=<ryubing>-ryurust.<version>`, so the version shows it's a
  Ryurust build.
- `-p:ExtraDefineConstants=DISABLE_UPDATER`: the updater is compiled out so
  the build can't replace itself with upstream Ryubing.

The `.so` files go in `lib/`, one of the paths the wrappers probe. The package
also contains both MIT licenses, `CREDITS.md`, and a short README
([`integration/ryubing/RELEASE-README.md`](../integration/ryubing/RELEASE-README.md)).

The script refuses to package `*.keys`, NCA/NSP/XCI/NRO files, or `portable/`
and `bis/` directories. Never build a release from a directory you've used to
play games.

## Requirements

- Rust stable.
- .NET SDK matching Ryubing's `global.json` (10.0.301 or a later 10.0 feature
  band). A user-local install works:
  `./dotnet-install.sh --channel 10.0 --install-dir ~/.dotnet`.
- `git`, `tar`, `xz`, network access to `git.ryujinx.app` and NuGet.

## Steps

1. Make sure `main` is green: fmt, clippy, tests, `cargo bench -- --test`.
2. Bump `version` in the crates you changed, and update
   `docs/KNOWN_ISSUES.md` and `docs/BENCHMARKS.md` if needed.
3. Build the package:

   ```bash
   export PATH="$HOME/.dotnet:$PATH"
   scripts/package-linux.sh 0.1.0
   ```

   Output: `dist/ryurust-0.1.0-ryubing-1.3.352-linux_x64.tar.xz` and its
   `.sha256`.
4. Smoke-test the package in a clean directory:
   - `sha256sum -c` the checksum file and extract the archive.
   - `mkdir portable` next to the executable (keeps the test out of your real
     config), run `./Ryujinx.sh`, and check the log in `portable/Logs/` shows
     the `<ryubing>-ryurust.<version>` version string and no errors.
   - Check every export the C# wrappers look up is present:
     `nm -D --defined-only lib/libryu_<name>.so`.
   - Load a game with your own keys and firmware and play for a few minutes.
   - Delete `portable/` before doing anything else with that directory.
5. Tag and publish:

   ```bash
   git tag -a v0.1.0 -m "Ryurust 0.1.0"
   git push origin v0.1.0
   gh release create v0.1.0 dist/ryurust-0.1.0-*.tar.xz dist/ryurust-0.1.0-*.sha256 \
       --title "Ryurust 0.1.0" --notes-file <notes>
   ```

## Updating the Ryubing base

1. Rebase the integration branch onto the new upstream tag and resolve
   conflicts.
2. Regenerate the patch:
   `git diff --full-index <new-base> HEAD -- src > integration/ryubing/ryurust-integration.patch`.
3. Update `integration/ryubing/BASE`.
4. Build and smoke-test as above.
