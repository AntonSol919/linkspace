#!/bin/bash -x 
set -euo pipefail
BINS="$(dirname "${BASH_SOURCE[0]}")"
GROUP=${GROUP:-"[#:pub]"}
BOARD=${1?Usage: board_name [start_stamp] }
magick convert -size 1000x1000 xc:transparent PNG32:$BOARD.png

# not strictly necessary, but otherwise pull does nothing
lk poll-status exchange $GROUP process --write "stdout-expr:exchange - [data]"  || echo "No exchange process active"

echo Pulling $GROUP $BOARD
lk pull "imageboard:$GROUP:/$BOARD" --follow -- "create:>:[now:-1D]" 

$BINS/imageboard.view.sh $BOARD 0 # run once

#On receiving a new packet of interest we repaint the board from that stamp
lk watch --new "imageboard:$GROUP:/$BOARD" | \
    lk printf "[create:str]" | \
    while read STAMP; do
        $BINS/imageboard.view.sh $BOARD $STAMP
    done
