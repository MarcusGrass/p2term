#!/bin/bash
set -ex

LTO="lto"
PACKAGE_DIR="target/packaged"
DEB_DIR="target/debian"
DEB_VER="0.1.0-1"
X86_MUSL="x86_64-unknown-linux-musl"
X86_GNU="x86_64-unknown-linux-gnu"
AARCH64_MUSL="aarch64-unknown-linux-musl"
AARCH64_GNU="aarch64-unknown-linux-gnu"
X86_WINDOWS="x86_64-pc-windows-gnu"
AARCH64_MAC="aarch64-apple-darwin"

function package() {
  BIN_NAME="$1"
  BUILD_TARGET="$2"
  OUTPUT_DIR="target/$BUILD_TARGET/$LTO"
  DEB_ARCH="$3"
  DEST_NAME="$4"
  EXT="$5"
  cross b -p "$BIN_NAME" --target "$BUILD_TARGET" --profile "$LTO"
  cp "$OUTPUT_DIR/$BIN_NAME""$EXT" "$PACKAGE_DIR/$DEST_NAME""$EXT"
  gpg --yes --armor --output "$PACKAGE_DIR/$DEST_NAME""$EXT.sig" --detach-sig "$PACKAGE_DIR/$DEST_NAME""$EXT"
  # Some shared artifacts end up here, getting reused and causing build failures
  rm -r "target/$LTO"
}

function package_deb() {
   package "$1" "$2" "$3" "$4" "$5"
   DEB_ARCH="$3"
   DEB_FILE="$DEB_DIR/$BIN_NAME"_"$DEB_VER"_"$DEB_ARCH.deb"
   cargo deb -p "$BIN_NAME" --target "$BUILD_TARGET" --profile "$LTO" --no-build --no-strip
   gpg --yes --armor --output "$DEB_FILE.sig" --detach-sig "$DEB_FILE"
   mv "$DEB_FILE.sig" "$PACKAGE_DIR"
   mv "$DEB_FILE" "$PACKAGE_DIR"
}

mkdir -p "$PACKAGE_DIR"
package_deb "p2termd" "$X86_MUSL" "amd64" "p2termd-x86_64-linux" ""
package_deb "p2termd" "$AARCH64_MUSL" "arm64" "p2termd-aarch64-linux" ""
package_deb "p2term" "$X86_MUSL" "amd64" "p2term-x86_64-linux" ""
package_deb "p2term" "$AARCH64_MUSL" "arm64" "p2term-aarch64-linux" ""

package "p2termd" "$X86_WINDOWS" "amd64" "p2termd" ".exe"
# Can't package client for windows before solving pty stuff
# package "p2term" "$X86_WINDOWS" "amd64" "p2term" ".exe"
