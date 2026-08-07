#!/bin/bash
set -e

ROOTFS_DIR="sysroot/aarch64-rootfs"
ALPINE_URL="https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/aarch64/alpine-minirootfs-3.20.2-aarch64.tar.gz"

echo "[*] Setting up Alpine Linux AArch64 RootFS..."

mkdir -p "$ROOTFS_DIR"
if [ ! -f "$ROOTFS_DIR/bin/busybox" ]; then
    echo "[*] Downloading Alpine AArch64 mini-rootfs tarball..."
    curl -sSL "$ALPINE_URL" | tar -xz -C "$ROOTFS_DIR"
    echo "[+] RootFS extracted to $ROOTFS_DIR"
else
    echo "[+] AArch64 RootFS is already initialized at $ROOTFS_DIR"
fi

echo ""
echo "[+] RootFS setup complete! You can now run any AArch64 binary inside $ROOTFS_DIR directly:"
echo "    ./sysroot/aarch64-rootfs/bin/busybox --list"
echo "    ./sysroot/aarch64-rootfs/bin/busybox uname -a"
