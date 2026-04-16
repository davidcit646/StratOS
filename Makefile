# StratOS Phase 1 Build System
PHASE1_SCRIPTS=scripts/phase1
OUT_DIR=out
DISK_IMAGE=$(OUT_DIR)/stratos-disk.raw
SYSTEM_IMAGE=$(OUT_DIR)/stratos-system.erofs

.PHONY: toolchain verify-toolchain system-image disk-image phase1 clean
.PHONY: verify-erofs-tooling verify-disk-layout-prereqs
.PHONY: install-supervisor stratterm strat-settings initramfs

# Phase 1: Toolchain & Build System
toolchain:
	@echo "=== Phase 1: Toolchain Setup ==="
	@$(PHASE1_SCRIPTS)/setup-toolchain.sh
	@echo "Phase 1 toolchain installation complete."

verify-toolchain:
	@echo "=== Phase 1: Toolchain Verification ==="
	@$(PHASE1_SCRIPTS)/verify-toolchain.sh

verify-erofs-tooling:
	@echo "=== Phase 1: EROFS Tooling Verification ==="
	@$(PHASE1_SCRIPTS)/verify-erofs-tooling.sh

verify-disk-layout-prereqs:
	@echo "=== Phase 1: Disk Layout Prerequisites Verification ==="
	@$(PHASE1_SCRIPTS)/verify-disk-layout-prereqs.sh

system-image:
	@echo "=== Phase 1: System Image Build ==="
	@$(PHASE1_SCRIPTS)/build-system-image.sh

disk-image:
	@echo "=== Phase 1: Disk Image Layout ==="
	@$(PHASE1_SCRIPTS)/create-disk-image.sh --image $(DISK_IMAGE) --size-gb 20

phase1: toolchain system-image disk-image
	@echo "=== Phase 1 Complete ==="
	@echo "Artifacts:"
	@echo "  System image: $(SYSTEM_IMAGE)"
	@echo "  Disk image: $(DISK_IMAGE)"

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
