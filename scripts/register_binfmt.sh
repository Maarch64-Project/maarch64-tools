#!/usr/bin/env bash
set -e

# Script to register maarch64 into Linux binfmt_misc for transparent AArch64 binary execution

RUNNER_PATH="$(readlink -f "$(dirname "$0")/../../target/debug/maarch64")"

if [ ! -f "$RUNNER_PATH" ]; then
    echo "[!] Error: maarch64 binary not found at $RUNNER_PATH"
    echo "    Please run 'cargo build -p maarch64-runner' first."
    exit 1
fi

BINFMT_MISC_DIR="/proc/sys/fs/binfmt_misc"
REGISTER_FILE="$BINFMT_MISC_DIR/register"
ENTRY_FILE="$BINFMT_MISC_DIR/maarch64"

if [ ! -d "$BINFMT_MISC_DIR" ]; then
    echo "[!] Error: binfmt_misc kernel module is not mounted or supported."
    exit 1
fi

# Check if entry already exists
if [ -f "$ENTRY_FILE" ]; then
    echo "[*] Unregistering existing maarch64 binfmt entry..."
    echo -1 | sudo tee "$ENTRY_FILE" > /dev/null || true
fi

echo "[*] Registering maarch64 binfmt handler..."
# AArch64 ELF Magic: \x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00 (header) + Machine 0x00b7 (183 = AArch64)
BINFMT_MAGIC=':maarch64:M::\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\xb7\x00:\xff\xff\xff\xff\xff\xff\xff\x00\xff\xff\xff\xff\xff\xff\xff\xff\xfe\xff\xff\xff:'"$RUNNER_PATH"':P'

echo "$BINFMT_MAGIC" | sudo tee "$REGISTER_FILE" > /dev/null

echo "[+] Successfully registered maarch64 as default AArch64 binary handler!"
echo "    You can now execute Linux AArch64 binaries directly (e.g. ./tests/bin/lua -v)."
