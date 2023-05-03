.PHONY: all debug common clean
R:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
LKB:=$(realpath $(R)/../target/)
RTOOL:=x86_64-unknown-linux-gnu
VERSION:=$(shell cat $(R)/../Cargo.toml | grep version | cut -d'"' -f2 )
PKG_DIR=$(R)/build/linkspace-$(VERSION)

all: aarch64 x64
x64: pkg-x86_64-unknown-linux-gnu
i686: pkg-x86_64-unknown-linux-musl # doesn't support python
aarch64: pkg-aarch64-unknown-linux-gnu

pkg-%: clean common cli-% py-%
	rm $(R)/build/linkspace-$(VERSION)-$*.zip || echo new
	cd $(R)/build && zip -r ./linkspace-$(VERSION)-$*.zip ./linkspace-$(VERSION)

clean:
	rm -fr $(PKG_DIR)

common:
	mkdir -p $(PKG_DIR)/bin
	rsync -rvkP --exclude "*private*" --exclude "_*" $(R)/../examples $(PKG_DIR)
	cp -r $(R)/../emulate $(PKG_DIR)
	cp $(R)/../connect-test-pub1 $(PKG_DIR)
	rsync -rvkP $(R)/common/ $(PKG_DIR)

cli-%:
	cd $(R)/.. && cross build --release --target $* -p linkspace-cli -p linkspace-handshake -p linkspace-lns
	cp $(LKB)/$*/release/lk $(PKG_DIR)/bin/
	cp $(LKB)/$*/release/linkspace-handshake $(PKG_DIR)/bin/
	cp $(LKB)/$*/release/linkspace-lns $(PKG_DIR)/bin/

py-%:
	mkdir -p $(PKG_DIR)/python/
	cd $(R)/.. && cross build --release --target $* -p linkspace-py
	cp $(LKB)/$*/release/liblinkspace.so $(PKG_DIR)/python/linkspace.so
	cp $(R)/../ffi/linkspace-py/linkspace.pyi $(PKG_DIR)/python/linkspace.pyi
