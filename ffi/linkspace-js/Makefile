.PHONY: build setup_env

build: # setup_env
	wasm-pack build --target web

watch:
	watchexec -w ./src make 
setup_env:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack


