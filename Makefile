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

rust-docs: 
	cargo +nightly doc -p linkspace --target-dir ./build --no-deps
	rsync -rvkP ./build/doc/ ./docs/cargo-doc

tutorials:
	make -C ./docs/tutorial/

# This requires the latest `lk` to be in path - for my setup build-debug is sufficient
guide: build-debug 
	make -C ./docs/guide/

docs: guide tutorials rust-docs

homepage:
	make -C ./homepage

git-checkin: homepage docs
	cargo +nightly check

publish: git-checkin
	rsync -rvkP --exclude "./homepage/.gitignore" ./homepage/ ./build/homepage
	git checkout publish
	rsync -rvkP ./build/homepage/ ./

