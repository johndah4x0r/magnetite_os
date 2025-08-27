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

all: $(BUILD_DIR) $(BUILD_DIR)/vbr.bin $(BUILD_DIR)/boot1.bin

clean:
	rm -r $(BUILD_DIR) || true
	cargo clean --manifest-path $(BOOT_RS_MANIFEST)
	cargo clean --manifest-path $(KERN_RS_MANIFEST)
	rm -f .patch_vbr*.log

bootimg: $(BUILD_DIR)/boot.img

debug_boot: $(BUILD_DIR)/boot.img
	bochs -q -f bochsrc

$(BUILD_DIR):
	mkdir -p $@

# --- Bootloader build process --- #
$(BUILD_DIR)/boot.img: $(BUILD_DIR) $(BUILD_DIR)/vbr.bin $(BUILD_DIR)/boot1.bin
	dd if=/dev/zero of=$@ bs=512 count=32768;
	mkfs.fat $@ \
		-F 16 \
		-M 0xf8 \
		-D 0x80 \
		-n "MAGNETITEOS" \
		-g 8/32 \
		-i 0x1337c0de \
		--mbr=yes;
	mcopy -i $@ $(BUILD_DIR)/boot1.bin ::/;
	./scripts/patch_vbr.sh --no-backup $@

$(BUILD_DIR)/vbr.bin: $(BOOT_SRC)/vbr.asm
	nasm $(BOOT_SRC)/vbr.asm -f bin -o $(BUILD_DIR)/vbr.bin 

$(BUILD_DIR)/stub32.o: $(BOOT_SRC)/stub16.asm $(BOOT_SRC)/stub32.asm
	nasm $(BOOT_SRC)/stub32.asm -f elf64 -o $(BUILD_DIR)/stub32.o

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
		./scripts/helper.sh $(BUILD_DIR)/boot_rs.o

# TODO:
# - add Rust bootloader object as dependency
# - use proper linker script
$(BUILD_DIR)/boot64.o: $(BUILD_DIR)/stub64.o $(BUILD_DIR)/boot_rs.o
	ld -m elf_x86_64 -T link_boot64.ld -r $^ -o $@

$(BUILD_DIR)/boot1.bin: $(BUILD_DIR)/stub32.o $(BUILD_DIR)/boot64.o
	ld -m elf_x86_64 -T link_boot1.ld --oformat=binary $^ -o $@

.PHONY: all clean bootimg debug_boot