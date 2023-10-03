#!/bin/bash -x 
set -euo pipefail
BINS="$(dirname "${BASH_SOURCE[0]}")"
LK_DOMAIN="imageboard" # set the default domain
LK_GROUP=${LK_GROUP:-"[#:pub]"}
BOARD=${1?Usage: board_name [start_stamp] }
magick convert -size 1000x1000 xc:transparent PNG32:$BOARD.png

# not strictly necessary, but otherwise pull does nothing
lk status watch exchange $LK_GROUP process --write "stdout-expr:exchange - [data]"  || echo "No exchange process active"

echo Pulling $LK_GROUP $BOARD
lk pull "imageboard:$LK_GROUP:/$BOARD" --follow -- "create:>:[now:-1D]" 

$BINS/imageboard.view.sh $BOARD 0 # run once

#On receiving a new packet of interest we repaint the board from that stamp
lk watch --new-only "imageboard:$LK_GROUP:/$BOARD" | \
    lk pktf "[create:str]" | \
    while read STAMP; do
        $BINS/imageboard.view.sh $BOARD $STAMP
    done

# we could use lk watch ::/$BOARD as both the LK_DOMAIN and LK_GROUP were set.
