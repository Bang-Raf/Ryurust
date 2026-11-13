#!/usr/bin/env bash
# Build a Windows x64 release of Ryubing with the Ryurust native libraries.
#
# Usage: scripts/package-windows.sh <ryurust-version> [ryubing-source-dir]
#
# Same steps as package-linux.sh, for win-x64: the integration patch is
# applied to upstream Ryubing, Ryubing is published self-contained for
# win-x64, the Rust crates are built as DLLs, and everything is packed into
# dist/ as a .zip with a .sha256.
#
# The Rust DLLs are built for x86_64-pc-windows-msvc with the C runtime
# linked statically, so they don't need the Visual C++ redistributable. That
# needs the MSVC toolchain, i.e. a Windows host (Git Bash works; this is what
# .github/workflows/windows.yml runs). On another host, build the DLLs on
# Windows first and point RYURUST_DLL_DIR at them; the .NET part
# cross-publishes from Linux like upstream's release workflow does.
#
# Requires: git, cargo (on Windows), .NET SDK matching Ryubing's global.json,
# 7z or zip, sha256sum.

set -euo pipefail

VERSION="${1:?usage: $0 <ryurust-version> [ryubing-source-dir]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INTEGRATION="$ROOT/integration/ryubing"
# shellcheck source=../integration/ryubing/BASE
source "$INTEGRATION/BASE"

TARGET=x86_64-pc-windows-msvc
APP_VERSION="${RYUBING_VERSION}-ryurust.${VERSION}"
PKG_NAME="ryurust-${VERSION}-ryubing-${RYUBING_VERSION}-win_x64"
BUILD="$ROOT/build"
SRC="${2:-$BUILD/ryubing-src}"
PUBLISH="$BUILD/publish-win"
DIST="$ROOT/dist"

echo "==> Ryurust $VERSION on Ryubing $RYUBING_TAG ($APP_VERSION), win-x64"

# 1. Ryubing source at the pinned base, with the integration patch applied.
if [ ! -d "$SRC/.git" ]; then
    git clone --depth 1 --branch "$RYUBING_TAG" "$RYUBING_REPO" "$SRC"
fi
actual="$(git -C "$SRC" rev-parse HEAD)"
if [ "$actual" != "$RYUBING_COMMIT" ]; then
    echo "error: $SRC is at $actual, expected $RYUBING_COMMIT ($RYUBING_TAG)" >&2
    exit 1
fi
if git -C "$SRC" apply --check "$INTEGRATION/ryurust-integration.patch" 2>/dev/null; then
    git -C "$SRC" apply "$INTEGRATION/ryurust-integration.patch"
elif git -C "$SRC" apply --reverse --check "$INTEGRATION/ryurust-integration.patch" 2>/dev/null; then
    echo "    integration patch already applied"
else
    echo "error: integration patch does not apply to $SRC" >&2
    exit 1
fi

# 2. Rust DLLs (or prebuilt ones from RYURUST_DLL_DIR).
if [ -n "${RYURUST_DLL_DIR:-}" ]; then
    DLL_DIR="$RYURUST_DLL_DIR"
    echo "    using prebuilt DLLs from $DLL_DIR"
else
    RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
        cargo build --manifest-path "$ROOT/Cargo.toml" --workspace --release --target "$TARGET"
    DLL_DIR="$ROOT/target/$TARGET/release"
fi
count="$(find "$DLL_DIR" -maxdepth 1 -name 'ryu_*.dll' | wc -l)"
if [ "$count" -ne 32 ]; then
    echo "error: expected 32 ryu_*.dll in $DLL_DIR, found $count" >&2
    exit 1
fi

# 3. Ryubing, as in its own release workflow, with the updater compiled out
#    so this build never replaces itself with upstream Ryubing.
rm -rf "$PUBLISH"
dotnet publish "$SRC/src/Ryujinx" -c Release -r win-x64 --self-contained \
    -o "$PUBLISH" \
    -p:Version="$APP_VERSION" \
    -p:SourceRevisionId="${RYUBING_COMMIT:0:9}" \
    -p:DebugType=embedded \
    -p:ExtraDefineConstants=DISABLE_UPDATER
rm -f "$PUBLISH/libarmeilleure-jitsupport.dylib"

# 4. Native libraries go in lib\, one of the paths the C# wrappers probe.
mkdir -p "$PUBLISH/lib"
cp "$DLL_DIR"/ryu_*.dll "$PUBLISH/lib/"

# 5. Licenses and notes.
cp "$SRC/LICENSE.txt" "$PUBLISH/LICENSE-Ryujinx.txt"
cp "$ROOT/LICENSE" "$PUBLISH/LICENSE-Ryurust.txt"
cp "$ROOT/CREDITS.md" "$PUBLISH/CREDITS.md"
cp "$INTEGRATION/RELEASE-README.md" "$PUBLISH/README-Ryurust.md"

# 6. Refuse to ship anything that must never be redistributed.
forbidden="$(find "$PUBLISH" \( -iname '*.keys' -o -iname '*.nca' -o -iname '*.nsp' \
    -o -iname '*.xci' -o -iname '*.nro' -o -name portable -o -name bis \) -print)"
if [ -n "$forbidden" ]; then
    echo "error: refusing to package keys, firmware or game files:" >&2
    echo "$forbidden" >&2
    exit 1
fi
if [ ! -f "$PUBLISH/Ryujinx.exe" ]; then
    echo "error: $PUBLISH/Ryujinx.exe was not produced" >&2
    exit 1
fi

# 7. Archive and checksum.
mkdir -p "$DIST"
rm -rf "${BUILD:?}/$PKG_NAME"
cp -a "$PUBLISH" "$BUILD/$PKG_NAME"
rm -f "$DIST/$PKG_NAME.zip"
if command -v 7z >/dev/null; then
    (cd "$BUILD" && 7z a -tzip -mx=9 "$DIST/$PKG_NAME.zip" "$PKG_NAME" >/dev/null)
else
    (cd "$BUILD" && zip -qr -9 "$DIST/$PKG_NAME.zip" "$PKG_NAME")
fi
(cd "$DIST" && sha256sum "$PKG_NAME.zip" > "$PKG_NAME.zip.sha256")

echo "==> $DIST/$PKG_NAME.zip"
cat "$DIST/$PKG_NAME.zip.sha256"
