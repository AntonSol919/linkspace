#!/bin/bash
set -euo pipefail

source "$(dirname -- "$0")/../activate"
cmd=${cmd:-anyhost.connect.sh}
name=${name:-${1:-bob}}

mkdir -p $name
cd $name
export LK_DIR=$PWD
export LK_PASS=$(lk key --no-pubkey --no-enckey --display-pass)
echo $name $(lk key --no-enckey --no-check) | tee ./emulate_name_key

tmux -S tmux-socket new-session -s $name -n $name \; \
     send-keys " lk " \; \
     split-window -v  \; \
     send-keys "lk --private watch :: --bare --new-only | lk pktf" C-m \; \
     split-window -v \; \
     send-keys "$cmd" C-m 
