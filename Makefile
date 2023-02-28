.PHONY: install-lk build build-lkpy docs homepage git-checkin publish

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

publish: git-checkin docs/guide/index.html
	rsync -rvrkP ./homepage/ ./build/homepage
	git rev-parse HEAD > ./build/PUBLISH_HEAD
	git checkout publish
	rsync -rvrkP ./build/homepage/ ./
	echo 'Publish Commit $(cat ./build/PUBLISH_HEAD)'

# ensure our index.html is up to date.
docs/guide/index.html: docs/guide/index.org
	echo "TODO: Currently not able to make guide/index.html outside of emacs"
	exit 1
