#!/bin/bash
set -euo pipefail

cmd=${cmd:-connect.sh}
name=${name:-${1:-bob}}
cd "$(dirname "$0")"
mkdir -p private/$name
source ./common
cd private/$name
export LINKSPACE=$PWD
tmux -S tmux-socket new-session -s $name -n $name \; \
     send-keys " lk " \; \
     split-window -v  \; \
     send-keys "lk --private watch :: --bare --mode log-asc | lk printf" C-m \; \
     split-window -v \; \
     send-keys "$cmd" C-m 
