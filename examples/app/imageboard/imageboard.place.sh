#!/bin/bash 
set -xeuo pipefail
if [ $# -lt 4 ]; then
    echo "Usage: img_file board_name X Y"
    exit 2
fi

IMG_FILE=$1; BOARD=$2; X=$3; Y=$4; 
shift 4

IMG_HASH=$(\
    cat $IMG_FILE \
        | lk data --write db --write stdout \
        | lk printf "[hash:str]")
TAG=$(printf "%08d%08d" $X $Y)

lk link "imageboard:$GROUP:/$BOARD" \
   -l $TAG:$IMG_HASH "$@" \
   --write db --write stdout \
    | lk printf
