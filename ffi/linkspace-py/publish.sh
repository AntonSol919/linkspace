#!/bin/bash
rm -rf ./.env
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain nightly
. $HOME/.cargo/env
make build
. .env/bin/activate
maturin publish --compatibility manylinux2014 --username Azon --password $PYPI
