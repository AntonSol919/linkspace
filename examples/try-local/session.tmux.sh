#!/bin/bash
set -euo pipefail
cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/.."
source ./common

cmd=${cmd:-connect.sh}
name=${name:-${1:-bob}}
mkdir -p private/$name
cd private/$name
export LK_DIR=$PWD
tmux -S tmux-socket new-session -s $name -n $name \; \
     send-keys " lk " \; \
     split-window -v  \; \
     send-keys "lk --private watch :: --bare --mode log-asc | lk printf" C-m \; \
     split-window -v \; \
     send-keys "$cmd" C-m 