#!/bin/bash
set -euo pipefail

export PORT=${PORT:-"5020"}
echo My Key $(lk key --password "" --insecure)
export GROUP=$(lk eval "{:$GROUP/?b}")
echo Serving $GROUP $PORT 

trap "kill -- -$$" EXIT

lk set-status exchange $GROUP process anyhost-client --data "abe:OK\nPID:$$\nwe're hosting" &

socat tcp-listen:$PORT,fork exec:"handshake.sh serve serve_io.sh",fdout=4


#websocat -e -E --binary --ping-timeout 15 --ping-interval 10 \
#         ws-l:0.0.0.0:5020 sh-c:"strace -e 'trace=!all' handshake.sh serve serve_io.sh"
