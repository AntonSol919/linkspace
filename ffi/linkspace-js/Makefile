.PHONY: build setup_env

build: CARGO_PROFILE_RELEASE_OPT_LEVEL="z" 
build: 
	@echo $(CARGO_PROFILE_RELEASE_OPT_LEVEL)
	wasm-pack build --target web

watch:
	watchexec -w ./src "rm ./pkg/* ; make"
setup_env:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack


