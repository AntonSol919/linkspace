# Examples

This folder contains two application examples and a proof of concept group exchange.
If you got this as the [pkg](https://antonsol919.github.io/linkspace/index.html#download) it comes bundled with the cli and python module.

Try and say hi by opening two shells:
In the first connect to a test server.
You'll need `socat` for this (available through most package managers).

```bash
./test-exchange
```

In the second run

```bash
source ./common
linkmail.py
```

## Testing locally

- ./try-local/host.session.tmux.sh creates a new instance and start an exchange.
- ./try-local/session.tmux.sh [NAME] creates a new instance and connect to the host.
