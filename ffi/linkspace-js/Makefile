.PHONY: build setup_env watch validate pack

build: setup_env
	wasm-pack build --target web

watch:
	watchexec -w ./src "rm ./pkg/prev.wasm ; cp ./pkg/linkspace_bg.wasm ./pkg/prev.wasm ; make"

# use with make watch to check for wasm bloat
watch_str:
	watchexec --no-vcs-ignore -w ./pkg/ -- ./check.sh


setup_env:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack

pack:
	rm -r ./pkg
	wasm-pack build
	jq '.files = ["*"]' ./pkg/package.json > ./pkg/tmp.json
	mv ./pkg/tmp.json ./pkg/package.json
	wasm-pack pack

validate:
	cargo check --target wasm32-unknown-unknown
	cargo clippy -- -D warnings
	test $$(cargo fmt --all -- --check | tee /dev/fd/2 | wc -c ) -eq 0
	cargo doc --all --no-deps
	cargo test
