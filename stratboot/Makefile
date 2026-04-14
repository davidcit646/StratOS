ARCH            ?= x86_64
CC              ?= $(firstword $(wildcard /run/host/usr/bin/x86_64-linux-gnu-gcc) x86_64-linux-gnu-gcc)
LD              ?= $(firstword $(wildcard /run/host/usr/bin/x86_64-linux-gnu-ld) x86_64-linux-gnu-ld)
OBJCOPY         ?= $(firstword $(wildcard /run/host/usr/bin/x86_64-linux-gnu-objcopy) x86_64-linux-gnu-objcopy)

GNUEFI_INCLUDE  ?= $(firstword $(wildcard /run/host/usr/include/efi /usr/include/efi))
GNUEFI_LIBDIR   ?= $(shell \
	if [ -f /run/host/usr/lib/libgnuefi.a ]; then echo /run/host/usr/lib; \
	elif [ -f /usr/lib/libgnuefi.a ]; then echo /usr/lib; \
	elif [ -f /usr/lib64/libgnuefi.a ]; then echo /usr/lib64; \
	else echo /usr/lib; fi)
GNUEFI_LDS      ?= $(firstword $(wildcard /run/host/usr/lib/elf_x86_64_efi.lds /usr/lib/elf_x86_64_efi.lds /usr/lib64/elf_x86_64_efi.lds))
GNUEFI_CRT      ?= $(firstword $(wildcard /run/host/usr/lib/crt0-efi-x86_64.o /usr/lib/crt0-efi-x86_64.o /usr/lib64/crt0-efi-x86_64.o))
SYSINCLUDE      ?= $(firstword $(wildcard /run/host/usr/include /usr/include))

CFLAGS  = -I$(GNUEFI_INCLUDE) -I$(GNUEFI_INCLUDE)/x86_64 \
          -I$(GNUEFI_INCLUDE)/protocol \
          -isystem $(SYSINCLUDE) \
          -fpic -fshort-wchar -mno-red-zone -maccumulate-outgoing-args \
          -fno-stack-protector \
          -DEFI_FUNCTION_WRAPPER -Wall -Wextra

LDFLAGS = -nostdlib -znocombreloc -T $(GNUEFI_LDS) \
          -shared -Bsymbolic \
          -L$(GNUEFI_LIBDIR)
LIBS    = -lefi -lgnuefi

SRCS = efi/strat_efi_vars.c src/gop.c src/font.c \
       src/input.c src/slot.c src/partition.c src/reset.c src/stratboot.c

OBJS = $(SRCS:.c=.o)

all: check BOOTX64.EFI

check:
	@if [ -z "$(GNUEFI_LDS)" ]; then echo "Missing elf_x86_64_efi.lds"; exit 1; fi
	@if [ -z "$(GNUEFI_CRT)" ]; then echo "Missing crt0-efi-x86_64.o"; exit 1; fi

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

stratboot.so: $(OBJS)
	$(LD) $(LDFLAGS) $(GNUEFI_CRT) $^ $(LIBS) -o $@

BOOTX64.EFI: stratboot.so
	$(OBJCOPY) -j .text -j .sdata -j .data -j .rodata \
	           -j .reloc \
	           --target=efi-app-x86_64 --subsystem=10 $< $@

clean:
	rm -f $(OBJS) stratboot.so BOOTX64.EFI

.PHONY: all clean check
