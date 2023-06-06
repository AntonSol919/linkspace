#!/bin/bash -x
# parent should set out to fd4. otherwise add a exec 4>&1 1>&2
set -euo pipefail
function fin (){
    kill -9 -- -$$ $(jobs -p) 2>/dev/null || true
    echo Disconnected - $THEIR_KEY 
}
trap "fin" EXIT

cd $SESSION
echo SESSION=$SESSION
echo THEIR_KEY=$THEIR_KEY
echo LK_GROUP=$LK_GROUP
LK_GROUP="[b:$LK_GROUP]"
THEIR_KEY="[b:$THEIR_KEY]"

lk link --create [u64:0] ":[#:0]:/rxlog/$THEIR_KEY" --write db
lk link --create [u64:0] ":[#:0]:/txlog/$THEIR_KEY" --write db
LAST_RX=$(lk --private watch --max 1 ":[#:0]:/rxlog/$THEIR_KEY" | lk printf [create:str])
LAST_TX=$(lk --private watch --max 1 ":[#:0]:/txlog/$THEIR_KEY" | lk printf [create:str])
lk eval "last rx [u64:$LAST_RX/s:str]\nlast tx [u64:$LAST_TX/s:str]\n"

lk set-status exchange $LK_GROUP process anyhost-client --read-str "$(lk e "OK\nPID:$$\nSESSION:$SESSION")" --read-repeat &

export LK_NO_CHECK=true

# save reads from stdin, ie. the server 
LK_NO_CHECK=false lk save --new db --new stdout \
    | lk printf --inspect "RX [domain:str] [path:str] [hash:str]" \
    | lk --private collect ":[#:0]:/rxlog/$THEIR_KEY" \
              --min-interval 1m \
              --forward null \
              --write db &

# read the pull request made by other apps and place them into the group
lk --private watch --new "[f:exchange]:[#:0]:/pull/$LK_GROUP:**" \
    | lk --private rewrite \
                --group $LK_GROUP \
                --write db --write stdout sign-all \
    | lk p  ">>>>new request [hash:str]\n[data]\n<<<<" &


# The exchange logic is to have every piece of data created locally send to a server
lk watch --bare --mode log-asc -- "group:=:$LK_GROUP" "hop:=:[u32:0]" "recv:>:[u64:$LAST_TX]" \
    | lk get-links skip \
    | lk dedup \
    | lk printf --inspect "[now:str] SENDING [hash:str]" \
    | tee --output-error=exit >( cat >&4 ) \
    | lk --private collect ":[#:0]:/txlog/$THEIR_KEY" \
         --min-interval 1m \
         --forward null \
         --write db &

echo PIDS $(jobs -p)
wait -n
