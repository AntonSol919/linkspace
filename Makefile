.PHONY: install-lk build build-python docs homepage git-checkin publish
R:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))

install-lk:
	cargo +nightly install --path ./cli/linkspace
	cargo +nightly install --path ./cli/handshake/
	cargo +nightly install --path ./cli/lns

build:
	cargo +nightly build -p linkspace

install-python:
	make -C ./ffi/linkspace-py install

build-debug:
	cargo +nightly build -p linkspace-cli -p linkspace-handshake -p linkspace-lns  -p linkspace-py
	rm -r "$(R)/target/python" || true
	mkdir -p "$(R)/target/python"
	ln -s "$(R)/target/debug/liblinkspace.so" "$(R)/target/python/linkspace.so" 
	ln -s "$(R)/ffi/linkspace-py/linkspace.pyi"  "$(R)/target/python/linkspace.pyi" 

validate: 
	cargo doc --all --no-deps
	cargo test
	cargo build
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
	make -C ./ffi/linkspace-js validate
