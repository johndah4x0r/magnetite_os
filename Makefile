BUILD_DIR := build
BOOT_SRC := boot/src

.DEFAULT_GOAL := all
.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/mbr.bin $(BUILD_DIR)/boot1.bin

clean:
	rm -r $(BUILD_DIR)

$(BUILD_DIR):
	mkdir -p $@

$(BUILD_DIR)/mbr.bin: $(BOOT_SRC)/mbr.asm
	nasm $(BOOT_SRC)/mbr.asm -f bin -o $(BUILD_DIR)/mbr.bin 

$(BUILD_DIR)/stub32.o: $(BOOT_SRC)/stub32.asm
	nasm $(BOOT_SRC)/stub32.asm -f elf32 -o $(BUILD_DIR)/stub32.o


$(BUILD_DIR)/stub64.o: $(BOOT_SRC)/stub64.asm
	nasm $(BOOT_SRC)/stub64.asm -f elf64 -o $(BUILD_DIR)/stub64.o 

# TODO:
# - add Rust bootloader object as dependency
# - use proper linker script
$(BUILD_DIR)/boot64.bin: $(BUILD_DIR)/stub64.o
	# TODO
	nasm $(BOOT_SRC)/stub64.asm -f bin -o $(BUILD_DIR)/boot64.bin

$(BUILD_DIR)/boot64_wrap.o: $(BUILD_DIR)/boot64.bin
	ld -r -m elf_i386 -b binary $(BUILD_DIR)/boot64.bin -o $(BUILD_DIR)/boot64_wrap.o;
	objcopy -S --rename-section .data=.w_text $(BUILD_DIR)/boot64_wrap.o;
	objcopy -S --add-symbol _start=.w_text:0 $(BUILD_DIR)/boot64_wrap.o


$(BUILD_DIR)/boot1.bin: $(BUILD_DIR)/stub32.o $(BUILD_DIR)/boot64_wrap.o
	ld -m elf_i386 -T link_boot1.ld --oformat=binary $(BUILD_DIR)/stub32.o $(BUILD_DIR)/boot64_wrap.o -o $(BUILD_DIR)/boot1.bin