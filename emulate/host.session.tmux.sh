#!/bin/bash
export name=alice
export cmd=anyhost.server.sh
exec "$(dirname "$0")/session.tmux.sh"
