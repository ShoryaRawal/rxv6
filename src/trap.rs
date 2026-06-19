use core::arch::global_asm;

const CLINT_MTIMECMP: *mut u64 = 0x0200_4000 as *mut u64;
const CLINT_MTIME: *const u64 = 0x0200_BFF8 as *const u64;
const INTERVAL: u64 = 10_000_000;

static mut TICKS: u64 = 0;

extern "C" {
    fn _trap_entry();
}

pub fn init() {
    unsafe {
        core::arch::asm!("csrw mtvec, {}", in(reg) _trap_entry as *const () as usize);
        core::arch::asm!("csrs mie, {}", in(reg) 1usize << 7);
        let now = CLINT_MTIME.read_volatile();
        CLINT_MTIMECMP.write_volatile(now + INTERVAL);
        core::arch::asm!("csrs mstatus, {}", in(reg) 1usize << 3);
    }
}

pub fn ticks() -> u64 {
    unsafe { core::ptr::read_volatile(&TICKS) }
}

#[no_mangle]
extern "C" fn _trap_rust() {
    let mcause: usize;
    unsafe { core::arch::asm!("csrr {}, mcause", out(reg) mcause) };
    let is_int = (mcause >> 63) & 1 == 1;
    let code = mcause & 0xff;
    if is_int && code == 7 {
        unsafe {
            TICKS += 1;
            let now = CLINT_MTIME.read_volatile();
            CLINT_MTIMECMP.write_volatile(now + INTERVAL);
        }
    }
}

global_asm!(
    ".align 4",
    ".globl _trap_entry",
    "_trap_entry:",
    "addi sp, sp, -128",
    "sd ra,   0(sp)",
    "sd t0,   8(sp)",
    "sd t1,  16(sp)",
    "sd t2,  24(sp)",
    "sd a0,  32(sp)",
    "sd a1,  40(sp)",
    "sd a2,  48(sp)",
    "sd a3,  56(sp)",
    "sd a4,  64(sp)",
    "sd a5,  72(sp)",
    "sd a6,  80(sp)",
    "sd a7,  88(sp)",
    "sd t3,  96(sp)",
    "sd t4, 104(sp)",
    "sd t5, 112(sp)",
    "sd t6, 120(sp)",
    "call _trap_rust",
    "ld ra,   0(sp)",
    "ld t0,   8(sp)",
    "ld t1,  16(sp)",
    "ld t2,  24(sp)",
    "ld a0,  32(sp)",
    "ld a1,  40(sp)",
    "ld a2,  48(sp)",
    "ld a3,  56(sp)",
    "ld a4,  64(sp)",
    "ld a5,  72(sp)",
    "ld a6,  80(sp)",
    "ld a7,  88(sp)",
    "ld t3,  96(sp)",
    "ld t4, 104(sp)",
    "ld t5, 112(sp)",
    "ld t6, 120(sp)",
    "addi sp, sp, 128",
    "mret",
);
