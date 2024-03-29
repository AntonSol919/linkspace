#!/bin/bash
# parent should set out to fd4. otherwise add a exec 4>&1 1>&2
set -euo pipefail
function fin (){
    kill -9 -- -$$ $(jobs -p) 2>/dev/null || true
    echo Disconnected - $THEIR_KEY 
}
trap "fin" EXIT

cd $SESSION

# ensure we have a fixed [b:...] repr (lns [#:name] could update between proc calls)
LK_GROUP=$(lk eval "$LK_GROUP" | lk encode b:32)
THEIR_KEY=$(lk eval "$THEIR_KEY" | lk encode b:32)

echo SESSION=$SESSION
echo THEIR_KEY=$THEIR_KEY
echo LK_GROUP=$LK_GROUP


lk link --create [u64:0] ":[#:0]:/rxlog/$THEIR_KEY" --write db
lk link --create [u64:0] ":[#:0]:/txlog/$THEIR_KEY" --write db
LAST_RX=$(lk --private watch --max 1 ":[#:0]:/rxlog/$THEIR_KEY" | lk pktf [create:str])
LAST_TX=$(lk --private watch --max 1 ":[#:0]:/txlog/$THEIR_KEY" | lk pktf [create:str])
lk eval "last rx [u64:$LAST_RX/us:str]\nlast tx [u64:$LAST_TX/us:str]\n"

lk status set exchange $LK_GROUP process anyhost-client --data-str "$(lk e "OK\nPID:$$\nSESSION:$SESSION")" --data-repeat &

export LK_SKIP_HASH=true

# save reads from stdin, ie. the server 
LK_SKIP_HASH=false lk save --new db --new stdout \
    | lk pktf --inspect "RX [domain:str] [spacename:str] [hash:str]" \
    | lk --private collect ":[#:0]:/rxlog/$THEIR_KEY" \
              --min-interval 1m \
              --forward null \
              --write db &

# read the pull request made by other apps and place them into the group
lk --private watch --new-only "[f:exchange]:[#:0]:/pull/$LK_GROUP:**" \
    | lk --private rewrite \
                --group $LK_GROUP \
                --write db --write stdout sign-all \
    | lk p  ">>>>new request [hash:str]\n[data]\n<<<<" &


# The exchange logic is to have every piece of data created locally send to a server
lk watch --bare --mode log-asc -- "group:=:$LK_GROUP" "hop:=:[u32:0]" "recv:>:[u64:$LAST_TX]" \
    | lk get-links skip \
    | lk dedup \
    | lk pktf --inspect "[now:str] SENDING [hash:str]" \
    | tee --output-error=exit >( cat >&4 ) \
    | lk --private collect ":[#:0]:/txlog/$THEIR_KEY" \
         --min-interval 1m \
         --forward null \
         --write db &

echo PIDS $(jobs -p)
wait -n
