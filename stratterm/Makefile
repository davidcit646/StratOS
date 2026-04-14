CARGO ?= cargo
ROOTFS_BIN_DIR ?= ../out/phase7/rootfs-minimal/bin

.PHONY: all build release run run-indexer run-indexer-once run-settings install-rootfs clean

all: build

build:
	$(CARGO) build --bins

release:
	$(CARGO) build --release --bins

run:
	GDK_BACKEND=wayland $(CARGO) run --bin stratterm

run-indexer:
	$(CARGO) run --bin stratterm-indexer -- --daemon

run-indexer-once:
	$(CARGO) run --bin stratterm-indexer -- --once

run-settings:
	GDK_BACKEND=wayland $(CARGO) run --bin strat-settings

install-rootfs: release
	mkdir -p "$(ROOTFS_BIN_DIR)"
	cp -f target/release/stratterm "$(ROOTFS_BIN_DIR)/stratterm"
	cp -f target/release/stratterm-indexer "$(ROOTFS_BIN_DIR)/stratterm-indexer"
	cp -f target/release/strat-settings "$(ROOTFS_BIN_DIR)/strat-settings"
	chmod 0755 "$(ROOTFS_BIN_DIR)/stratterm"
	chmod 0755 "$(ROOTFS_BIN_DIR)/stratterm-indexer"
	chmod 0755 "$(ROOTFS_BIN_DIR)/strat-settings"

clean:
	$(CARGO) clean
