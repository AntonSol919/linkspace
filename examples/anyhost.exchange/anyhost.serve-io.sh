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
echo SESSION=$SESSION 
echo THEIR_KEY=$THEIR_KEY
echo LK_GROUP=$LK_GROUP
echo PID=$PID
LK_GROUP="[b:$LK_GROUP]"
THEIR_KEY="[b:$THEIR_KEY]"

lk link --create [u64:0] ":[#:0]:/rxlog/$THEIR_KEY" --write db
lk link --create [u64:0] ":[#:0]:/txlog/$THEIR_KEY" --write db
LAST_RX=$(lk --private watch --max 1 ":[#:0]:/rxlog/$THEIR_KEY" | lk printf [create:str])
LAST_TX=$(lk --private watch --max 1 ":[#:0]:/txlog/$THEIR_KEY" | lk printf [create:str])
lk eval "last rx [u64:$LAST_RX/s:str]\nlast tx [u64:$LAST_TX/s:str]\n"

export LK_SKIP_HASH=true
# save reads from std. i.e. what the client is sending
LK_SKIP_HASH=false lk save --new db --new stdout \
        --old file:>( lk printf "$PID Ignored [hash:str] (old)" >&2 ) \
   | lk printf --inspect "$PID RX [domain:str] [path:str] [hash:str]" \
   | lk --private collect ":[#:0]:/rxlog/$THEIR_KEY" \
        --min-interval 1m \
        --forward null \
        --write db  > /dev/null &

# Read new request keypoints and return their content
lk watch --new "[f:exchange]:$LK_GROUP:/pull/$LK_GROUP:**" -- "pubkey:=:$THEIR_KEY"  \
    | lk printf --inspect ">>>>Pull req [hash:str]\n[data]\n<<<<$PID " \
    | lk multi-watch \
    | lk dedup \
    | lk printf --inspect "$PID Tx [hash:str]" >&4 


echo PIDS $(jobs -p)
wait -n
