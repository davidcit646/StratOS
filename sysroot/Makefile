# sysroot Makefile - Initramfs build

CC ?= musl-gcc
CFLAGS = -static -Wall -Wextra -O2
INITRAMFS_DIR = $(OUT_DIR)/initramfs
INITRAMFS_CPIO = $(OUT_DIR)/initramfs.cpio
INITRAMFS_GZ = $(OUT_DIR)/initramfs.cpio.gz

OUT_DIR = out

.PHONY: initramfs clean

initramfs: $(INITRAMFS_GZ)

$(INITRAMFS_GZ): $(INITRAMFS_CPIO)
	@echo "Compressing initramfs with gzip..."
	@gzip -f -c $(INITRAMFS_CPIO) > $(INITRAMFS_GZ)
	@echo "Initramfs built: $(INITRAMFS_GZ)"

$(INITRAMFS_CPIO): initramfs-init.c
	@echo "Building static init binary..."
	@mkdir -p $(INITRAMFS_DIR)
	@$(CC) $(CFLAGS) -o $(INITRAMFS_DIR)/init initramfs-init.c
	@echo "Creating cpio archive (newc format)..."
	@cd $(INITRAMFS_DIR) && find . | cpio -o -H newc > ../../$(INITRAMFS_CPIO)
	@echo "Initramfs archive created: $(INITRAMFS_CPIO)"

clean:
	@echo "Cleaning sysroot build artifacts..."
	@rm -rf $(OUT_DIR)
	@echo "Clean complete."
