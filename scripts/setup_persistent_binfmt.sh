#!/usr/bin/env bash
set -e

# Script to make maarch64 binfmt_misc handler persistent across reboots via /etc/binfmt.d/

RUNNER_PATH="$(readlink -f "$(dirname "$0")/../../target/debug/maarch64")"

if [ ! -f "$RUNNER_PATH" ]; then
    echo "[!] Error: maarch64 binary not found at $RUNNER_PATH"
    echo "    Please run 'cargo build -p maarch64-runner' first."
    exit 1
fi

BINFMT_CONF="/etc/binfmt.d/maarch64.conf"
BINFMT_MAGIC=':maarch64:M::\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\xb7\x00:\xff\xff\xff\xff\xff\xff\xff\x00\xff\xff\xff\xff\xff\xff\xff\xff\xfe\xff\xff\xff:'"$RUNNER_PATH"':P'

echo "[*] Writing persistent binfmt configuration to $BINFMT_CONF..."
echo "$BINFMT_MAGIC" | sudo tee "$BINFMT_CONF" > /dev/null

echo "[*] Restarting systemd-binfmt service..."
sudo systemctl restart systemd-binfmt || true

echo "[+] Success! maarch64 is now persistently registered and will automatically activate on every boot."
