#!/bin/bash
set -euo pipefail

export PORT=${PORT:-"5020"}
echo My Key $(lk key)
export GROUP=$(lk eval "[:$GROUP/?b]")
echo Serving $GROUP $PORT 
export LINKSPACE_PASS=$(lk key --no-pubkey --no-enckey --display-pass)

function fin (){
    kill -9 -- -$$
    kill -9 -- $(jobs -p) 2>/dev/null || true
    echo Disconnected - $GROUP $PORT
}
trap "fin" EXIT

lk set-status exchange $GROUP process anyhost-client --data "abe:OK\nPID:$$\nwe're hosting" &

socat tcp-listen:$PORT,fork exec:"handshake.sh serve serve_io.sh",fdout=4 &

echo PIDS $(jobs -p)
wait -n

#websocat -e -E --binary --ping-timeout 15 --ping-interval 10 \
#         ws-l:0.0.0.0:5020 sh-c:"strace -e 'trace=!all' handshake.sh serve serve_io.sh"
