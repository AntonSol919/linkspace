#!/bin/bash
set -euo pipefail
if [[ ${SOCAT_PEERADDR+x} ]]
   then
    export THEIR_ADDR=$SOCAT_PEERADDR:$SOCAT_PEERPORT
else
    # websocat
    export THEIR_ADDR=${WEBSOCAT_CLIENT:-$SERVER}
fi
export SESSION=$(mktemp -dt $THEIR_ADDR.XXXXX)

MODE=${1:-serve}
lk handshake --max-diff-secs 6000 --password "$LINKSPACE_PASS" \
          --write stdout --write file:$SESSION/handshake.out \
          --forward file:$SESSION/handshake.in \
          $MODE >&4

export THEIR_KEY=$(cat $SESSION/handshake.in | lk filter --bare --signed --max-new 1 | lk printf "[pubkey:str]")
echo Connected $THEIR_ADDR - Their Key : $THEIR_KEY 1>&2 
exec ${@:2} 
