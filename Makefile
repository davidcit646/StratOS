# StratOS Phase 1 Build System
PHASE0_SCRIPTS=scripts/phase0
OUT_DIR=out
DISK_IMAGE=$(OUT_DIR)/stratos-disk.raw

.PHONY: toolchain disk-image clean
.PHONY: install-supervisor stratterm strat-settings initramfs

# Phase 1: Toolchain & Build System
toolchain:
	@echo "Installing Rust UEFI target..."
	@$(PHASE0_SCRIPTS)/install-toolchain.sh
	@echo "Installing GNU-EFI libraries..."
	@$(PHASE0_SCRIPTS)/install-gnu-efi.sh
	@echo "Phase 1 toolchain installation complete."

disk-image:
	@echo "Creating QEMU disk image with GPT partition layout..."
	@$(PHASE0_SCRIPTS)/create-qemu-disk-image.sh --image $(DISK_IMAGE) --size-gb 20
	@echo "Phase 1 disk image complete."

clean:
	@echo "Removing out/ directory..."
	@rm -rf $(OUT_DIR)
	@echo "Clean complete."

# Later phase targets
install-supervisor:
	@if [ ! -f "$(SUPERVISOR_SRC)" ]; then \
		echo "Missing supervisor binary: $(SUPERVISOR_SRC)" >&2; \
		exit 1; \
	fi
	@install -d /boot/efi/strat
	@install -m 0755 "$(SUPERVISOR_SRC)" "$(SUPERVISOR_DST)"
	@echo "Installed supervisor to $(SUPERVISOR_DST)"

stratterm:
	$(MAKE) -C stratterm build

strat-settings:
	$(MAKE) -C stratterm run-settings

initramfs:
	$(MAKE) -C sysroot initramfs
