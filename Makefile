.PHONY: all clean
	# Boo!

all: mbr.bin boot1.bin

clean:
	# FIXME: This is *extremely* dangerous!
	rm -f boot*.bin boot*.o mbr.bin

(boot/src/mbr.asm boot/src/stub32.asm boot/src/stu64.asm): Makefile

mbr.bin: boot/src/mbr.asm
	nasm boot/src/mbr.asm -f bin -o mbr.bin 

stub32.o: boot/src/stub32.asm
	nasm boot/src/stub32.asm -f elf32 -o stub32.o


stub64.o: boot/src/stub64.asm
	nasm boot/src/stub64.asm -f elf64 -o stub64.o 

# TODO:
# - add Rust bootloader object as dependency
# - use proper linker script
boot64.bin: stub64.o
	# TODO
	nasm boot/src/stub64.asm -f bin -o boot64.bin

boot64_wrap.o: boot64.bin
	ld -r -m elf_i386 -b binary boot64.bin -o boot64_wrap.o;
	objcopy --rename-section .data=.w_text boot64_wrap.o

boot1.bin: stub32.o boot64_wrap.o
	ld -m elf_i386 -T link_boot1.ld --oformat=binary stub32.o boot64_wrap.o -o boot1.bin