
# Emulate local group exchange

Requires tmux.

Two scripts to start a very simple (insecure) group exchange. 
They default to using $PWD/$name, meaning you can run them from anywhere (e.g. /tmp/emulate)
Begin with running the `host.session.tmux.sh` script, then connect one or more `session.tmux.sh [NAME]`. 
The `/examples/*` directories are added to the $PATH. 
Meaning you can use an application like `linkmail.py` directly.
