#!/bin/bash
export name=alice
export cmd=serve.sh
exec "$(dirname "$0")/client.session.tmux.sh"
