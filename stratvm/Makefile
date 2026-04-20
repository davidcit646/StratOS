CC ?= cc
PKG_CONFIG ?= pkg-config
WAYLAND_SCANNER ?= wayland-scanner

# .pc locations: Debian/Ubuntu use /usr/lib/pkgconfig/wlroots-0.17.pc.
# Arch installs /usr/lib/wlroots0.17/pkgconfig/wlroots.pc (package name "wlroots", not "wlroots-0.17").
ARCH_WLR_PKGCONFIG := $(subst $(space),:,$(wildcard /usr/lib/wlroots0.*/pkgconfig))
# Fedora/RHEL use /usr/lib64/pkgconfig; Debian uses /usr/lib/x86_64-linux-gnu/pkgconfig
export PKG_CONFIG_PATH := $(PKG_CONFIG_PATH):/usr/lib/pkgconfig:/usr/lib64/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/local/lib/pkgconfig:/usr/local/lib64/pkgconfig:$(ARCH_WLR_PKGCONFIG)

# Prefer libraries installed to ~/.local (e.g. Meson --prefix=$HOME/.local after building wlroots from source).
STRATVM_USE_LOCAL_PREFIX ?= 0
ifeq ($(STRATVM_USE_LOCAL_PREFIX),1)
export PKG_CONFIG_PATH := $(HOME)/.local/share/pkgconfig:$(HOME)/.local/lib/x86_64-linux-gnu/pkgconfig:$(HOME)/.local/lib/pkgconfig:$(PKG_CONFIG_PATH)
export PATH := $(HOME)/.local/bin:$(PATH)
endif

# Optional: link against wlroots built from the git submodule (see target wlroots-build).
# Usage: make wlroots-build && make STRATVM_BUNDLED_WLROOTS=1
STRATVM_BUNDLED_WLROOTS ?= 0
WLROOTS_SRC := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/wlroots)
# pkg-config stub for hwdata (wlroots DRM); Ubuntu 22.04 ships pnp.ids without hwdata.pc.
STRATVM_PKGCONFIG_STUBS := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/pkgconfig-stubs)
ifeq ($(STRATVM_BUNDLED_WLROOTS),1)
# Prefer uninstalled .pc so -L/-I point at the in-tree build (meson-private alone uses prefix=/usr/local).
export PKG_CONFIG_PATH := $(WLROOTS_SRC)/build/meson-uninstalled:$(WLROOTS_SRC)/build/meson-private:$(PKG_CONFIG_PATH)
endif

ifeq ($(shell command -v $(PKG_CONFIG) >/dev/null 2>&1 && echo ok),)
$(error pkg-config not found — install pkgconf: Arch: sudo pacman -S pkgconf | Debian: sudo apt install pkg-config)
endif

# Prefer newest compatible wlroots; bare "wlroots" last (Arch's .pc is named wlroots.pc inside wlroots0.17/).
# Fallback: any wlroots*.pc under /usr/lib/pkgconfig or Arch versioned pkgconfig dirs.
# $(shell ...) does not always inherit Makefile exports; set PKG_CONFIG_PATH explicitly here.
WLROOTS_PKG := $(shell \
	P="$(PKG_CONFIG_PATH)"; \
	if [ "$(STRATVM_BUNDLED_WLROOTS)" = "1" ]; then \
		P="$(WLROOTS_SRC)/build/meson-uninstalled:$(WLROOTS_SRC)/build/meson-private:$$P"; \
	fi; \
	export PKG_CONFIG_PATH="$$P"; \
	pkg_ok() { \
		_pkg="$$1"; \
		$(PKG_CONFIG) --exists "$$_pkg" >/dev/null 2>&1 || return 1; \
		$(PKG_CONFIG) --cflags "$$_pkg" >/dev/null 2>&1 || return 1; \
		$(PKG_CONFIG) --libs "$$_pkg" >/dev/null 2>&1 || return 1; \
		return 0; \
	}; \
	if [ "$(STRATVM_BUNDLED_WLROOTS)" = "1" ]; then \
		for p in wlroots-0.19-uninstalled wlroots-0.19; do \
			if pkg_ok "$$p"; then \
				echo "$$p"; exit 0; \
			fi; \
		done; \
		exit 1; \
	fi; \
	for p in wlroots-0.22 wlroots-0.21 wlroots-0.20 wlroots-0.19 wlroots-0.18 wlroots-0.17 wlroots; do \
		pkg_ok "$$p" && echo "$$p" && exit 0; \
	done; \
	for f in /usr/lib/pkgconfig/wlroots*.pc /usr/lib64/pkgconfig/wlroots*.pc /usr/lib/wlroots0.*/pkgconfig/wlroots.pc; do \
		[ -f "$$f" ] || continue; \
		b=$$(basename "$$f" .pc); \
		pkg_ok "$$b" && echo "$$b" && exit 0; \
	done; \
	exit 1)

ifeq ($(strip $(WLROOTS_PKG)),)
$(error wlroots pkg-config not found — install wlroots (Arch: sudo pacman -S wlroots0.17) and pkgconf. \
	Arch uses pkg name "wlroots" under /usr/lib/wlroots0.17/pkgconfig/ — this Makefile adds that to PKG_CONFIG_PATH.)
endif

# Header root must match the library pkg-config selects (avoid mixing trees from wildcards).
# Meson wlroots uses Cflags: -I${includedir}/wlroots-0.19 (not plain includedir); prefer that path.
# Installed wlroots uses -I.../wlroots-0.19; in-tree / uninstalled uses -I.../wlroots/include (avoid matching .../wlroots/build/include first).
WLR_INCLUDEDIR := $(shell $(PKG_CONFIG) --cflags-only-I $(WLROOTS_PKG) 2>/dev/null | tr ' ' '\n' | grep '^-I.*/wlroots-0\.[0-9][0-9]*$$' | head -1 | sed 's/^-I//')
ifeq ($(strip $(WLR_INCLUDEDIR)),)
WLR_INCLUDEDIR := $(shell $(PKG_CONFIG) --cflags-only-I $(WLROOTS_PKG) 2>/dev/null | tr ' ' '\n' | grep '^-I.*/wlroots/include$$' | head -1 | sed 's/^-I//')
endif
ifeq ($(strip $(WLR_INCLUDEDIR)),)
WLR_INCLUDEDIR := $(shell $(PKG_CONFIG) --variable=includedir $(WLROOTS_PKG) 2>/dev/null)
endif
ifeq ($(strip $(WLR_INCLUDEDIR)),)
WLR_COMPOSITOR_H := $(firstword $(sort $(wildcard \
	/usr/include/wlroots-0.*/wlr/types/wlr_compositor.h \
	/usr/include/wlroots0.*/wlr/types/wlr_compositor.h \
	/usr/include/wlroots/wlr/types/wlr_compositor.h \
	$(HOME)/.local/include/wlroots-0.*/wlr/types/wlr_compositor.h)))
ifeq ($(WLR_COMPOSITOR_H),)
WLR_COMPOSITOR_H := $(shell find $(HOME)/.local/include /usr/include /usr/local/include -path '*/wlr/types/wlr_compositor.h' 2>/dev/null | head -1)
endif
WLR_INCLUDEDIR := $(patsubst %/wlr/types/wlr_compositor.h,%,$(WLR_COMPOSITOR_H))
endif
ifeq ($(strip $(WLR_INCLUDEDIR)),)
$(error wlroots headers not found — $(PKG_CONFIG) has no includedir for $(WLROOTS_PKG), and no .../wlr/types/wlr_compositor.h under /usr/include. \
	Install wlroots dev headers (Debian: libwlroots-dev | Arch: wlroots0.17).)
endif
ifeq ($(wildcard $(WLR_INCLUDEDIR)/wlr/types/wlr_compositor.h),)
$(error wlroots compositor header missing at $(WLR_INCLUDEDIR)/wlr/types/wlr_compositor.h — broken or partial install)
endif
# Ensure the compiler sees this tree first (pkg-config --cflags also adds -I; duplicate is harmless).
CPPFLAGS += -I$(WLR_INCLUDEDIR)
ifneq ($(wildcard $(WLR_INCLUDEDIR)/wlr/types/wlr_subcompositor.h),)
CPPFLAGS += -DSTRATWM_HAVE_WLR_SUBCOMPOSITOR=1
endif

WL_PROTO_DIR := $(shell $(PKG_CONFIG) --variable=pkgdatadir wayland-protocols 2>/dev/null)
ifeq ($(strip $(WL_PROTO_DIR)),)
WL_PROTO_DIR := /usr/share/wayland-protocols
endif

# swaywm/wlr-protocols XMLs for wayland-scanner — not always registered in pkg-config.
# Override: make WLR_PROTO_DIR=/path/to/wlr-protocols
WLR_PROTO_CAND := $(shell $(PKG_CONFIG) --variable=pkgdatadir wlr-protocols 2>/dev/null)
ifeq ($(strip $(WLR_PROTO_CAND)),)
WLR_PROTO_CAND := $(firstword $(wildcard \
	/usr/share/wlr-protocols \
	/usr/local/share/wlr-protocols \
	$(HOME)/.local/share/wlr-protocols))
endif
WLR_PROTO_DIR ?= $(WLR_PROTO_CAND)
WLR_LAYER_XML := $(WLR_PROTO_DIR)/unstable/wlr-layer-shell-unstable-v1.xml
ifeq ($(wildcard $(WLR_LAYER_XML)),)
$(error wlr-protocols not found (need $(WLR_LAYER_XML)). Fedora: dnf install wlr-protocols-devel \
	(not wlr-protocols). Else: git clone --depth=1 https://gitlab.freedesktop.org/wlroots/wlr-protocols.git \
	/usr/local/share/wlr-protocols — or set WLR_PROTO_DIR to the clone path)
endif
PKGS := $(WLROOTS_PKG) wayland-server xkbcommon pixman-1 libinput libevdev

CFLAGS += -std=c11 -Wall -Wextra -Wpedantic -g
CPPFLAGS += -DWLR_USE_UNSTABLE $(shell $(PKG_CONFIG) --cflags $(PKGS)) -I.
LDLIBS += $(shell $(PKG_CONFIG) --libs $(PKGS)) -lm

PROTO_HEADERS := xdg-shell-protocol.h wlr-layer-shell-unstable-v1-protocol.h
SRC := src/main.c
OBJ := $(SRC:.c=.o)
BIN := stratwm
ROOTFS_BIN_DIR ?= ../out/phase7/rootfs-minimal/bin

.PHONY: all clean install wlroots-build

all: $(BIN)

# Build wlroots from ./wlroots (submodule), then: make STRATVM_BUNDLED_WLROOTS=1
wlroots-build:
	cd "$(WLROOTS_SRC)" && rm -rf build && \
	rm -rf subprojects/v4l-utils subprojects/packagecache/v4l-utils-*.tar* 2>/dev/null; \
	rm -f subprojects/v4l-utils.wrap subprojects/edid-decode.wrap; \
	PKG_CONFIG_PATH="$(STRATVM_PKGCONFIG_STUBS):$(PKG_CONFIG_PATH)" \
	meson setup build \
		-Dexamples=false \
		-Dwerror=false \
		-Dbackends=drm,libinput \
		-Dsession=enabled \
		-Drenderers=auto \
		-Dallocators=auto \
		--buildtype=release \
		--wrap-mode=default && \
	meson compile -C build

xdg-shell-protocol.h:
	$(WAYLAND_SCANNER) server-header $(WL_PROTO_DIR)/stable/xdg-shell/xdg-shell.xml $@

wlr-layer-shell-unstable-v1-protocol.h:
	$(WAYLAND_SCANNER) server-header $(WLR_LAYER_XML) $@

src/main.o: $(PROTO_HEADERS)

# Explicit rule so CPPFLAGS (wlroots -I and -D) are never dropped vs default %.o rules.
%.o: %.c
	$(CC) $(CPPFLAGS) $(CFLAGS) -c -o $@ $<

$(BIN): $(OBJ)
	$(CC) $(CFLAGS) $(OBJ) -o $@ $(LDLIBS)

clean:
	rm -f $(OBJ) $(BIN) $(PROTO_HEADERS)

install: $(BIN)
	mkdir -p $(ROOTFS_BIN_DIR)
	cp -f $(BIN) $(ROOTFS_BIN_DIR)/$(BIN)
	chmod 0755 $(ROOTFS_BIN_DIR)/$(BIN)
