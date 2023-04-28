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

docs:
	cargo +nightly doc -p linkspace --target-dir ./build --no-deps
	cp -r ./build/doc/ ./docs/cargo-doc

homepage:
	make -C ./homepage

homepage-downloads:
	rm -r ./homepage/download
	mkdir -p ./homepage/download
	make -C ./pkg all
	cp ./pkg/build/*.zip ./homepage/download


git-checkin: homepage docs
	cargo +nightly check

publish: git-checkin docs/guide/index.html
	rsync -rvkP ./homepage/ ./build/homepage
	git rev-parse HEAD > ./build/PUBLISH_HEAD
	git checkout publish
	rsync -rvkP ./build/homepage/ ./
	echo 'Publish Commit $(cat ./build/PUBLISH_HEAD)'

# ensure our index.html is up to date.
docs/guide/index.html: docs/guide/index.org
	echo "TODO: Currently not able to make guide/index.html outside of emacs"
	exit 1
