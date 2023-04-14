#!/bin/bash
export name=alice
export cmd=serve.sh
exec "$(dirname "$0")/session.tmux.sh"
