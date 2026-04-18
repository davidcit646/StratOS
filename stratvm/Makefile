CC ?= cc
PKG_CONFIG ?= pkg-config
WAYLAND_SCANNER ?= wayland-scanner

WLROOTS_PKG := $(shell \
	if $(PKG_CONFIG) --exists wlroots; then echo wlroots; \
	elif $(PKG_CONFIG) --exists wlroots-0.19; then echo wlroots-0.19; \
	elif $(PKG_CONFIG) --exists wlroots-0.18; then echo wlroots-0.18; \
	elif $(PKG_CONFIG) --exists wlroots-0.17; then echo wlroots-0.17; \
	else echo; fi)

ifeq ($(strip $(WLROOTS_PKG)),)
$(error wlroots pkg-config entry not found (wlroots / wlroots-0.19 / wlroots-0.18 / wlroots-0.17))
endif

WL_PROTO_DIR := $(shell $(PKG_CONFIG) --variable=pkgdatadir wayland-protocols)
WLR_PROTO_DIR := $(shell $(PKG_CONFIG) --variable=pkgdatadir wlr-protocols 2>/dev/null)
ifeq ($(strip $(WLR_PROTO_DIR)),)
WLR_PROTO_DIR := /usr/share/wlr-protocols
endif
PKGS := $(WLROOTS_PKG) wayland-server xkbcommon pixman-1 libinput

CFLAGS += -std=c11 -Wall -Wextra -Wpedantic -g
CPPFLAGS += -DWLR_USE_UNSTABLE $(shell $(PKG_CONFIG) --cflags $(PKGS)) -I.
LDLIBS += $(shell $(PKG_CONFIG) --libs $(PKGS)) -lm

PROTO_HEADERS := xdg-shell-protocol.h wlr-layer-shell-unstable-v1-protocol.h
SRC := src/main.c
OBJ := $(SRC:.c=.o)
BIN := stratwm
ROOTFS_BIN_DIR ?= ../out/phase7/rootfs-minimal/bin

.PHONY: all clean install

all: $(BIN)

xdg-shell-protocol.h:
	$(WAYLAND_SCANNER) server-header $(WL_PROTO_DIR)/stable/xdg-shell/xdg-shell.xml $@

wlr-layer-shell-unstable-v1-protocol.h:
	$(WAYLAND_SCANNER) server-header $(WLR_PROTO_DIR)/unstable/wlr-layer-shell-unstable-v1.xml $@

src/main.o: $(PROTO_HEADERS)

$(BIN): $(OBJ)
	$(CC) $(CFLAGS) $(OBJ) -o $@ $(LDLIBS)

clean:
	rm -f $(OBJ) $(BIN) $(PROTO_HEADERS)

install: $(BIN)
	mkdir -p $(ROOTFS_BIN_DIR)
	cp -f $(BIN) $(ROOTFS_BIN_DIR)/$(BIN)
	chmod 0755 $(ROOTFS_BIN_DIR)/$(BIN)
