.PHONY: build setup_env

build:
	wasm-pack build --target web

watch:
	watchexec -w ./src "rm ./pkg/prev.wasm ; cp ./pkg/linkspace_bg.wasm ./pkg/prev.wasm ; make"

# use with make watch to check for wasm bloat
watch_str:
	watchexec --no-vcs-ignore -w ./pkg/ -- ./check.sh


setup_env:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack
