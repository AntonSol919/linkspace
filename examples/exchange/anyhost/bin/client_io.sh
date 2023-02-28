#!/bin/bash
# parent should set out to fd4. otherwise add a exec 4>&1 1>&2
set -euo pipefail
function fin (){
    kill $(jobs -p) 2>/dev/null || true
    echo Disconnected - $THEIR_KEY 
}
trap "fin" EXIT

cd $SESSION
echo SESSION=$SESSION
echo THEIR_KEY=$THEIR_KEY
echo GROUP=$GROUP
GROUP="[b:$GROUP]"
THEIR_KEY="[b:$THEIR_KEY]"

lk link --create [u64:0] ":[#:0]:/rxlog/$THEIR_KEY" --write db
lk link --create [u64:0] ":[#:0]:/txlog/$THEIR_KEY" --write db
LAST_RX=$(lk --private watch --max 1 ":[#:0]:/rxlog/$THEIR_KEY" | lk printf [create:str])
LAST_TX=$(lk --private watch --max 1 ":[#:0]:/txlog/$THEIR_KEY" | lk printf [create:str])
lk eval "last rx [u64:$LAST_RX/s:str]\nlast tx [u64:$LAST_TX/s:str]\n"

lk set-status exchange $GROUP process anyhost-client --data "abe:OK\nPID:$$\nSESSION:$SESSION" &

# save reads from stdin, ie. the server 
lk save --new db --new stdout \
    | lk printf --inspect "RX [domain:str] [path:str] [hash:str]" \
    | lk --private collect ":[#:0]:/rxlog/$THEIR_KEY" \
              --min-interval 1m \
              --forward null \
              --write db &

# read the pull request made by other apps and place them into the group
lk --private watch --new "[f:exchange]:[#:0]:/pull/$GROUP:**" \
    | lk --private rewrite \
                --password "" \
                --group $GROUP \
                --write db --write stdout sign-all \
    | lk p  ">>>>new request [hash:str]\n[data]\n<<<<" &


# This group exchange requires us to send all the data to the server
lk watch --bare --mode log-asc -- "group:=:$GROUP" "hop:=:[:0/u32]" \
    | lk get-links \
    | lk dedup \
    | lk printf --inspect "[now:str] SENDING [hash:str]" \
    | tee --output-error=exit >( cat >&4 ) \
    | lk --private collect ":[#:0]:/txlog/$THEIR_KEY" \
         --min-interval 1m \
         --forward null \
         --write db &

echo PIDS $(jobs -p)
wait -n
