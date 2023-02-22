.PHONY: install-lk build build-lkpy docs homepage

install-lk:
	cargo +nightly install --path ./cli/linkspace
	cargo +nightly install --path ./cli/handshake/

build:
	cargo +nightly build -p liblinkspace

install-lkpy:
	make -C ./ffi/liblinkspace-py install

docs:
	cargo +nightly doc -p liblinkspace --target-dir ./build --no-deps
	cp -r ./build/doc ./docs/cargo-doc

homepage:
	make -C ./homepage

git-checkin: homepage docs
	cargo +nightly check -p liblinkspace
	cargo +nightly check -p linkspace-cli #./cli/linkspace
	cargo +nightly check -p lkpy          #./ffi/liblinkspace-py

publish: git-checkin
	rsync -rvrkP ./homepage/ ./build/homepage
