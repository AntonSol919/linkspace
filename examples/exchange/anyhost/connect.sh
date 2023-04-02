#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
lk key --password ${PASS:-""}
export SERVER=${SERVER:-${1:-"127.0.0.1:5020"}}
export GROUP_ORIG=$GROUP
export GROUP=$(lk eval "[:$GROUP/?b]")
echo Connecting $GROUP $SERVER

socat tcp:$SERVER,keepalive exec:"handshake.sh connect client_io.sh",fdout=4
#websocat -E --binary ws://$SERVER sh-c:"handshake.sh connect client_io.sh"
