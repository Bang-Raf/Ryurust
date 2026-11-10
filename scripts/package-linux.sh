#!/usr/bin/env bash
# Build a Linux x64 release of Ryubing with the Ryurust native libraries.
#
# Usage: scripts/package-linux.sh <ryurust-version> [ryubing-source-dir]
#
# Without a source dir, upstream Ryubing is cloned at the pinned base tag.
# The integration patch in integration/ryubing/ is applied, Ryubing is
# published self-contained, the Rust crates are built in release mode, and
# everything is packed into dist/.
#
# Requires: git, cargo, .NET SDK matching Ryubing's global.json, tar, xz.

set -euo pipefail

VERSION="${1:?usage: $0 <ryurust-version> [ryubing-source-dir]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INTEGRATION="$ROOT/integration/ryubing"
# shellcheck source=../integration/ryubing/BASE
source "$INTEGRATION/BASE"

APP_VERSION="${RYUBING_VERSION}-ryurust.${VERSION}"
PKG_NAME="ryurust-${VERSION}-ryubing-${RYUBING_VERSION}-linux_x64"
BUILD="$ROOT/build"
SRC="${2:-$BUILD/ryubing-src}"
PUBLISH="$BUILD/publish"
DIST="$ROOT/dist"

echo "==> Ryurust $VERSION on Ryubing $RYUBING_TAG ($APP_VERSION)"

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

# 2. Rust libraries.
cargo build --manifest-path "$ROOT/Cargo.toml" --workspace --release

# 3. Ryubing, as in its own release workflow, with the updater compiled out
#    so this build never replaces itself with upstream Ryubing.
rm -rf "$PUBLISH"
dotnet publish "$SRC/src/Ryujinx" -c Release -r linux-x64 --self-contained \
    -o "$PUBLISH" \
    -p:Version="$APP_VERSION" \
    -p:SourceRevisionId="${RYUBING_COMMIT:0:9}" \
    -p:DebugType=embedded \
    -p:ExtraDefineConstants=DISABLE_UPDATER
rm -f "$PUBLISH/libarmeilleure-jitsupport.dylib"

# 4. Native libraries go in lib/, one of the paths the C# wrappers probe.
mkdir -p "$PUBLISH/lib"
cp "$ROOT"/target/release/libryu_*.so "$PUBLISH/lib/"

# 5. Licenses and notes.
cp "$SRC/LICENSE.txt" "$PUBLISH/LICENSE-Ryujinx.txt"
cp "$ROOT/LICENSE" "$PUBLISH/LICENSE-Ryurust.txt"
cp "$ROOT/CREDITS.md" "$PUBLISH/CREDITS.md"
cp "$INTEGRATION/RELEASE-README.md" "$PUBLISH/README-Ryurust.md"
chmod +x "$PUBLISH/Ryujinx" "$PUBLISH/Ryujinx.sh"

# 6. Refuse to ship anything that must never be redistributed.
forbidden="$(find "$PUBLISH" \( -iname '*.keys' -o -iname '*.nca' -o -iname '*.nsp' \
    -o -iname '*.xci' -o -iname '*.nro' -o -name portable -o -name bis \) -print)"
if [ -n "$forbidden" ]; then
    echo "error: refusing to package keys, firmware or game files:" >&2
    echo "$forbidden" >&2
    exit 1
fi

# 7. Archive and checksum.
mkdir -p "$DIST"
rm -rf "${BUILD:?}/$PKG_NAME"
cp -a "$PUBLISH" "$BUILD/$PKG_NAME"
tar -C "$BUILD" -cJf "$DIST/$PKG_NAME.tar.xz" "$PKG_NAME"
(cd "$DIST" && sha256sum "$PKG_NAME.tar.xz" > "$PKG_NAME.tar.xz.sha256")

echo "==> $DIST/$PKG_NAME.tar.xz"
cat "$DIST/$PKG_NAME.tar.xz.sha256"
