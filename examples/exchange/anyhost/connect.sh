#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
lk key 
export SERVER=${SERVER:-${1:-"127.0.0.1:5020"}}
export GROUP_ORIG=$GROUP
export GROUP=$(lk eval "[:$GROUP/?b]")

socat /dev/null tcp:$SERVER
# We could check if the server is responding first, but if we do we're on the clock during the handshake.
# so instead we save the password here
export LINKSPACE_PASS=$(lk key --no-pubkey --no-enckey --display-pass)

echo Connecting $GROUP $SERVER
socat tcp:$SERVER,keepalive exec:"handshake.sh connect client_io.sh",fdout=4
#websocat -E --binary ws://$SERVER sh-c:"handshake.sh connect client_io.sh"
