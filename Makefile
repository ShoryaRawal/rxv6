TARGET := riscv64gc-unknown-none-elf
KERNEL := target/$(TARGET)/release/rxv6
QEMU   := qemu-system-riscv64

QEMU_FLAGS := -machine virt -bios none -kernel $(KERNEL) -device virtio-gpu-device -device virtio-keyboard-device -vga none -serial stdio

.PHONY: build run debug clean image

build:
	cargo build --release

run: build
	$(QEMU) $(QEMU_FLAGS)

debug: build
	$(QEMU) $(QEMU_FLAGS) -s -S &
	@echo "GDB on :1234 — attach with gdb-multiarch $(KERNEL)"

clean:
	cargo clean
