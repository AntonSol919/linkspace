#!/bin/bash
set -euo pipefail
BOARD=${1?Usage: board_name [start_stamp] }
if [[ ! -f $BOARD.png ]]; then
    magick convert -size 1000x1000 xc:transparent PNG32:$BOARD.png # Create empty canvas
fi
START_STAMP=${2:-"0"} # If no stamp is given we begin at 0, i.e. unix epoch in microseconds

# We select everything with a create field greater or equal to $START_STAMP
lk watch --index "imgboard:$GROUP:/$BOARD" -- "create:>=:{u64:$START_STAMP}" \
    | lk printf "{/links:{tag:str} {ptr:str}}" \
    | while read REF; do
        X=${REF:0:8}
        Y=${REF:8:8}
        IMG_HASH=${REF: -43}
        echo "Placing $IMG_HASH at $X , $Y"
        lk watch-hash $IMG_HASH \
            | lk printf "{data}" --delimiter "" \
            | magick composite -geometry +$X+$Y - PNG32:$BOARD.png PNG32:$BOARD.png
    done
echo "$BOARD: $START_STAMP"
