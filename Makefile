TARGET := riscv64gc-unknown-none-elf
KERNEL := target/$(TARGET)/release/rxv6
QEMU   := qemu-system-riscv64

QEMU_FLAGS := -machine virt -nographic -bios none -kernel $(KERNEL)

.PHONY: build run debug clean

build:
	cargo build --release

run: build
	$(QEMU) $(QEMU_FLAGS)

debug: build
	$(QEMU) $(QEMU_FLAGS) -s -S &
	@echo "GDB on :1234 — attach with gdb-multiarch $(KERNEL)"

clean:
	cargo clean
