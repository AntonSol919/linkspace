#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
export SERVER=${SERVER:-${1:-"127.0.0.1:5020"}}
export LK_GROUP_ORIG=$LK_GROUP
export LK_GROUP=$(lk eval "[:$LK_GROUP/?b]")

socat /dev/null tcp:$SERVER
export LK_PASS=$(lk key --no-pubkey --no-enckey --display-pass)

echo Connecting $LK_GROUP $SERVER
socat tcp:$SERVER,keepalive exec:"handshake.sh connect client_io.sh",fdout=4
#websocat -E --binary ws://$SERVER sh-c:"handshake.sh connect client_io.sh"
