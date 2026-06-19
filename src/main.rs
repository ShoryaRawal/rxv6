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

use core::arch::global_asm;

global_asm!(
    ".section .text.boot",
    ".globl _start",
    "_start:",
    "    csrr t0, mhartid",
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
    "    call kmain",
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
    trap::init();
    fs::init();

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
