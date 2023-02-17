#!/bin/bash -x 
set -euo pipefail
GROUP=${GROUP:-"{#:pub}"}
BOARD=${1?Usage: board_name [start_stamp] }
magick convert -size 1000x1000 xc:transparent PNG32:$BOARD.png

function pull {
    echo Pulling $GROUP $BOARD
    lk pull --ttl 10m "imgboard:$GROUP:/$BOARD" -- "create:>:{now:-1D}" "create:<:{now:+1h}" 
    sleep 600
    pull
}

./imgboard-view.sh $BOARD 0 # run once

#On receiving a new packet of interest we repaint the board from that stamp
lk view --new "imgboard:$GROUP:/$BOARD" | \
    lk printf "{create:str}" | \
    while read STAMP; do
        ./imgboard-view.sh $BOARD $STAMP
    done
