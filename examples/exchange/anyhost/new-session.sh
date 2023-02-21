#!/bin/bash
set -euo pipefail

cmd=${cmd:-connect.sh}
name=${name:-${1:-bob}}
cd "$(dirname "$0")"
mkdir -p private/$name
source ./common
cd private/$name
export LINKSPACE=$PWD
build-cli

tmux new-session -s $name -n $name \
     -e PATH="$PATH" \
     -e LINKSPACE="$PWD" -e GROUP="$GROUP" \; \
     send-keys "build-lk ; lk " \; \
     split-window -v  \; \
     send-keys "lk --private watch :: --bare --mode log-asc | lk printf" C-m \; \
     split-window -v \; \
     send-keys "build-lk ; $cmd" C-m 
