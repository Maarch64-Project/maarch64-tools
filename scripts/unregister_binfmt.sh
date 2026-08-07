#!/usr/bin/env bash
set -e

# Script to unregister maarch64 from Linux binfmt_misc

ENTRY_FILE="/proc/sys/fs/binfmt_misc/maarch64"

if [ -f "$ENTRY_FILE" ]; then
    echo "[*] Unregistering maarch64 binfmt handler..."
    echo -1 | sudo tee "$ENTRY_FILE" > /dev/null
    echo "[+] maarch64 binfmt entry successfully removed."
else
    echo "[*] maarch64 binfmt entry is not currently registered."
fi
