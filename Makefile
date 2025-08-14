.DEFAULT_GOAL := all

BUILD_DIR := build
BOOT_SRC := boot/src
KERN_SRC := kern/src

TARGET_TRIPLET := x86_64-unknown-none
TARGET_SPEC := target_specs/$(TARGET_TRIPLET).json

BOOT_RS_MANIFEST := boot/Cargo.toml
KERN_RS_MANIFEST := kern/Cargo.toml

BOOT_RS_DIR := boot/target/$(TARGET_TRIPLET)/release/deps
BOOT_RS_GLOB := boot-*.o

KERN_RS_DIR := kern/target/$(TARGET_TRIPLET)/release/deps
KERN_RS_GLOB := kern-*.o

all: $(BUILD_DIR) $(BUILD_DIR)/mbr.bin $(BUILD_DIR)/boot1.bin

clean:
	rm -r $(BUILD_DIR) || true
	cargo clean --manifest-path $(BOOT_RS_MANIFEST)
	cargo clean --manifest-path $(KERN_RS_MANIFEST)

debug_boot: $(BUILD_DIR)/boot.img
	bochs -q -f bochsrc

$(BUILD_DIR):
	mkdir -p $@

# --- Bootloader build process --- #
$(BUILD_DIR)/boot.img: $(BUILD_DIR) $(BUILD_DIR)/mbr.bin $(BUILD_DIR)/boot1.bin
	dd if=/dev/zero of=$(BUILD_DIR)/boot.img bs=512 count=16;
	dd if=$(BUILD_DIR)/mbr.bin of=$(BUILD_DIR)/boot.img bs=512 conv=notrunc
	dd if=$(BUILD_DIR)/boot1.bin of=$(BUILD_DIR)/boot.img bs=512 seek=1 conv=notrunc

$(BUILD_DIR)/mbr.bin: $(BOOT_SRC)/mbr.asm
	nasm $(BOOT_SRC)/mbr.asm -f bin -o $(BUILD_DIR)/mbr.bin 

$(BUILD_DIR)/stub32.o: $(BOOT_SRC)/stub32.asm
	nasm $(BOOT_SRC)/stub32.asm -f elf32 -o $(BUILD_DIR)/stub32.o

$(BUILD_DIR)/stub64.o: $(BOOT_SRC)/stub64.asm
	nasm $(BOOT_SRC)/stub64.asm -f elf64 -o $(BUILD_DIR)/stub64.o 

# Rust routines
# Sequential command list
# 1. Build Rust-based ELF linkable object
# 2. Copy latest file into the target file
# FIXME: This WILL NOT detect dependencies,
# and WILL run 'cargo' unconditionally
$(BUILD_DIR)/boot_rs.o: $(shell find $(BOOT_SRC) -type f -name '*.rs')
	cargo +nightly rustc \
		--release \
		-Z build-std=core,compiler_builtins \
		--target $(TARGET_SPEC) \
		--manifest-path $(BOOT_RS_MANIFEST) \
		-- --emit=obj;

	find $(BOOT_RS_DIR) \
		-type f -name $(BOOT_RS_GLOB) \
		-exec stat -c "%Y %n" {} + | \
		sort -nr | \
		awk 'NR==1 { print $$2 }' | \
		./helper.sh $(BUILD_DIR)/boot_rs.o

# TODO:
# - add Rust bootloader object as dependency
# - use proper linker script
$(BUILD_DIR)/boot64.bin: $(BUILD_DIR)/stub64.o $(BUILD_DIR)/boot_rs.o
	ld -m elf_x86_64 -T link_boot64.ld --oformat=binary $^ -o $@

$(BUILD_DIR)/boot64_wrap.o: $(BUILD_DIR)/boot64.bin
	ld -r -m elf_i386 -b binary $(BUILD_DIR)/boot64.bin -o $(BUILD_DIR)/boot64_wrap.o;
	objcopy -S --rename-section .data=.w_text $(BUILD_DIR)/boot64_wrap.o;
	objcopy --add-symbol _start=.w_text:0 $(BUILD_DIR)/boot64_wrap.o

$(BUILD_DIR)/boot1.bin: $(BUILD_DIR)/stub32.o $(BUILD_DIR)/boot64_wrap.o
	ld -m elf_i386 -T link_boot1.ld --oformat=binary $^ -o $@

.PHONY: all clean debug_boot