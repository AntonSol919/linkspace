#!/bin/bash
set -euo pipefail

export PORT=${PORT:-"5020"}
echo My Key $(lk key)
export LK_GROUP=$(lk eval "[:$LK_GROUP/?b]")
echo Serving $LK_GROUP $PORT 
export LK_PASS=$(lk key --no-pubkey --no-enckey --display-pass)

function fin (){
    kill -9 -- -$$
    kill -9 -- $(jobs -p) 2>/dev/null || true
    echo Disconnected - $LK_GROUP $PORT
}
trap "fin" EXIT

lk set-status exchange $LK_GROUP process anyhost-client --read-str "$(lk e "OK\nPID:$$\nwe're hosting")" &

socat tcp-listen:$PORT,fork exec:"anyhost.handshake.sh serve anyhost.serve-io.sh",fdout=4 &

echo PIDS $(jobs -p)
wait -n

#websocat -e -E --binary --ping-timeout 15 --ping-interval 10 \
#         ws-l:0.0.0.0:5020 sh-c:"strace -e 'trace=!all' handshake.sh serve serve_io.sh"
