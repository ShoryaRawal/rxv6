#![no_std]
#![no_main]
#![allow(static_mut_refs)]
#![allow(dead_code)]

mod uart;
mod console;
mod alloc;
mod trap;
mod fs;
mod shell;
mod editor;
mod paging;
mod virtio;
mod gpu;
mod font;
mod keyboard;

use core::arch::global_asm;

global_asm!(
    ".section .text.boot",
    ".globl _start",
    "_start:",
    "    mv t0, a0",
    "    bnez t0, _park",
    "    la   sp, _stack_top",
    "    la   t0, _bss_start",
    "    la   t1, _bss_end",
    "_bss_clear:",
    "    bgeu t0, t1, _bss_done",
    "    sd   zero, 0(t0)",
    "    addi t0, t0, 8",
    "    j    _bss_clear",
    "_bss_done:",
    "    li t0, 0x3fffffffffffffff",
    "    csrw pmpaddr0, t0",
    "    li t0, 0x1f",
    "    csrw pmpcfg0, t0",
    "    li t0, 1 << 63",
    "    csrw 0x30a, t0", // 0x30A is menvcfg
    "    li t0, -1",
    "    csrw mcounteren, t0",
    "    li t0, 0xffff",
    "    csrw medeleg, t0",
    "    csrw mideleg, t0",
    "    li t0, 1 << 11",
    "    csrw mstatus, t0",
    "    la t0, kmain",
    "    csrw mepc, t0",
    "    mret",
    "_park:",
    "    wfi",
    "    j    _park",
);

#[no_mangle]
extern "C" fn kmain() -> ! {
    uart::init();

    println!();
    println!("=== RxV6 Operating System ===");
    println!("Version 0.1.0 | RISC-V 64-bit");
    println!();

    alloc::init();
    println!("Alloc init done");
    trap::init();
    println!("Trap init done");
    paging::init();
    println!("Paging init done");
    fs::init();
    println!("FS init done");
    keyboard::init();
    println!("Keyboard init done");

    if gpu::init() {
        println!("GPU initialized. Framebuffer: {}x{}", gpu::WIDTH, gpu::HEIGHT);
        console::clear_screen();
    } else {
        println!("No VirtIO GPU found.");
    }

    println!("Boot complete. Type 'help' for available commands.");
    println!();

    shell::run();
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!();
    println!("!!! KERNEL PANIC !!!");
    println!("{}", info);
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
