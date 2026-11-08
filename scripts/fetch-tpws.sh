#!/bin/sh
set -e
V=v72.13
SUM=441524a76f52597a0e5c816d25ed1adfd93e5c34cc56c56e0dc00533929d1761
D=$(mktemp -d)
curl -fsSL -o "$D/zapret.tar.gz" "https://github.com/bol-van/zapret/releases/download/$V/zapret-$V.tar.gz"
tar -xzf "$D/zapret.tar.gz" -C "$D" "zapret-$V/binaries/mac64/tpws"
echo "$SUM  $D/zapret-$V/binaries/mac64/tpws" | shasum -a 256 -c -
cp "$D/zapret-$V/binaries/mac64/tpws" src-tauri/engine/tpws
chmod 755 src-tauri/engine/tpws
lipo -info src-tauri/engine/tpws
rm -rf "$D"
