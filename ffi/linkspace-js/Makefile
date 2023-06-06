.PHONY: build setup_env

build: # setup_env
	wasm-pack build --target web

setup_env:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack


