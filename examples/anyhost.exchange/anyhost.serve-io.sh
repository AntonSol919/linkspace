#!/bin/bash
# parent should set out to fd4. otherwise add a exec 4>&1 1>&2
set -euo pipefail
PID=$$
function fin (){
    kill -9 -- -$$ $(jobs -p) 2>/dev/null || true
    echo $PID Disconnected - $THEIR_KEY 
}
trap "fin" EXIT

cd $SESSION

# ensure we have a fixed [b:...] repr (lns [#:name] could update between proc calls)
LK_GROUP=$(lk eval "$LK_GROUP" | lk encode b:32)
THEIR_KEY=$(lk eval "$THEIR_KEY" | lk encode b:32)

echo SESSION=$SESSION 
echo THEIR_KEY=$THEIR_KEY
echo LK_GROUP=$LK_GROUP
echo PID=$PID

lk link --create [u64:0] ":[#:0]:/rxlog/$THEIR_KEY" --write db
lk link --create [u64:0] ":[#:0]:/txlog/$THEIR_KEY" --write db
LAST_RX=$(lk --private watch --max 1 ":[#:0]:/rxlog/$THEIR_KEY" | lk pktf [create:str])
LAST_TX=$(lk --private watch --max 1 ":[#:0]:/txlog/$THEIR_KEY" | lk pktf [create:str])
lk eval "last rx [u64:$LAST_RX/us:str]\nlast tx [u64:$LAST_TX/us:str]\n"

export LK_SKIP_HASH=true
# save reads from std. i.e. what the client is sending
LK_SKIP_HASH=false lk save --new db --new stdout \
        --old file:>( lk pktf "$PID Ignored [hash:str] (old)" >&2 ) \
   | lk pktf --inspect "$PID RX [domain:str] [spacename:str] [hash:str]" \
   | lk --private collect ":[#:0]:/rxlog/$THEIR_KEY" \
        --min-interval 1m \
        --forward null \
        --write db  > /dev/null &

# Read new request keypoints and return their content
lk watch --new-only "[f:exchange]:$LK_GROUP:/pull/$LK_GROUP:**" -- "pubkey:=:$THEIR_KEY"  \
    | lk pktf --inspect ">>>>Pull req [hash:str]\n[data]\n<<<<$PID " \
    | lk multi-watch \
    | lk dedup \
    | lk pktf --inspect "$PID Tx [hash:str]" >&4 


echo PIDS $(jobs -p)
wait -n
