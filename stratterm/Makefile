CARGO ?= cargo
ROOTFS_BIN_DIR ?= ../out/phase7/rootfs-minimal/bin

.PHONY: all build release release-indexer run run-indexer run-indexer-once run-settings install-rootfs clean

all: build

build:
	$(CARGO) build --bins

release:
	$(CARGO) build --release --bins

release-indexer:
	$(CARGO) build --release --no-default-features --bin stratterm-indexer

run:
	GDK_BACKEND=wayland $(CARGO) run --bin stratterm

run-indexer:
	$(CARGO) run --no-default-features --bin stratterm-indexer -- --daemon

run-indexer-once:
	$(CARGO) run --no-default-features --bin stratterm-indexer -- --once

run-settings:
	GDK_BACKEND=wayland $(CARGO) run --bin strat-settings

install-rootfs: release-indexer
	mkdir -p "$(ROOTFS_BIN_DIR)"
	cp -f target/release/stratterm-indexer "$(ROOTFS_BIN_DIR)/stratterm-indexer"
	chmod 0755 "$(ROOTFS_BIN_DIR)/stratterm-indexer"
	@if pkg-config --exists gtk4; then \
		$(CARGO) build --release --bin stratterm --bin strat-settings; \
		cp -f target/release/stratterm "$(ROOTFS_BIN_DIR)/stratterm"; \
		cp -f target/release/strat-settings "$(ROOTFS_BIN_DIR)/strat-settings"; \
		chmod 0755 "$(ROOTFS_BIN_DIR)/stratterm" "$(ROOTFS_BIN_DIR)/strat-settings"; \
	else \
		echo "Skipping GUI binaries (gtk4.pc not found): stratterm, strat-settings"; \
	fi

clean:
	$(CARGO) clean
