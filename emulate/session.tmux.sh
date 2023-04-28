#!/bin/bash
set -euo pipefail
cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/.."
cmd=${cmd:-anyhost.connect.sh}
name=${name:-${1:-bob}}

mkdir -p private/$name
cd private/$name
export LK_DIR=$PWD

export LK_PASS=$(lk key --no-pubkey --no-enckey --display-pass --password "")
echo $name $(lk key --no-enckey --no-check) | tee ./name_key

tmux -S tmux-socket new-session -s $name -n $name \; \
     send-keys " lk " \; \
     split-window -v  \; \
     send-keys "lk --private watch :: --bare --new | lk printf" C-m \; \
     split-window -v \; \
     send-keys "$cmd" C-m 
