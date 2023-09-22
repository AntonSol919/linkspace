#!/bin/bash
set -euo pipefail
exec 4>&1 1>&2
trap "trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT


# HACK: could gen random ubits0 to avoid echo ub0=$(cat /dev/random | head -c 4 | lk encode u32)

lk p --inspect "recv [pkt] " | lk filter $* --write db --write-false stdout -- "spacename:=:$WEBSOCAT_URI" | \
    lk p "Received invalid packet [pkt]" &

lk watch $* -- "spacename:=:$WEBSOCAT_URI" >&4

wait -n
