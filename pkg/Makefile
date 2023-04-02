.PHONY: all debug common clean
R:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
LKB:=$(realpath $(R)/../target/)
RTOOL:=x86_64-unknown-linux-gnu
PKG_DIR=$(R)/build/linkspace-pkg
# cargo doesn't take --debug so make sure you update the rules as well. 
RTARGET:=release

all: x64 aarch64
x64: pkg-x86_64-unknown-linux-gnu
i686: pkg-x86_64-unknown-linux-musl # doesn't support lkpy
aarch64: pkg-aarch64-unknown-linux-gnu

pkg-%: clean common cli-% py-%
	rm $(R)/build/linkspace-$*.zip || echo new
	cd $(R)/build && zip -r ./linkspace-$*.zip ./linkspace-pkg

clean:
	rm -fr $(PKG_DIR)

common:
	mkdir -p $(PKG_DIR)/bin
	rsync -rvkP --exclude "*private*" --exclude "imageboard-rs" $(R)/../examples/ $(PKG_DIR)

cli-%:
	cd $(R)/.. && cross build --$(RTARGET) --target $* -p linkspace-cli -p linkspace-handshake -p linkspace-lns
	cp $(LKB)/$*/$(RTARGET)/lk $(PKG_DIR)/bin/
	cp $(LKB)/$*/$(RTARGET)/linkspace-handshake $(PKG_DIR)/bin/
	cp $(LKB)/$*/$(RTARGET)/linkspace-lns $(PKG_DIR)/bin/

py-%:
	cd $(R)/.. && cross build --$(RTARGET) --target $* -p lkpy
	cp $(LKB)/$*/$(RTARGET)/liblkpy.so $(PKG_DIR)/lkpy.so
	cp $(R)/../ffi/liblinkspace-py/lkpy.pyi $(PKG_DIR)/