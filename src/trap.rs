use core::arch::global_asm;

const INTERVAL: u64 = 10_000_000;

static mut TICKS: u64 = 0;

core::arch::global_asm!(include_str!("vector.S"));

extern "C" {
    fn _vector_table();
}

fn sbi_set_timer(stime_value: u64) {
    unsafe {
        // Use stimecmp CSR (0x14D) instead of SBI
        core::arch::asm!("csrw 0x14d, {}", in(reg) stime_value);
    }
}

fn read_time() -> u64 {
    let time: u64;
    unsafe { core::arch::asm!("csrr {}, time", out(reg) time) };
    time
}

pub fn init() {
    unsafe {
        // Set stvec to vector table, mode 1 (vectored)
        core::arch::asm!("csrw stvec, {}", in(reg) (_vector_table as *const () as usize | 1));
        // Enable Supervisor Timer Interrupt (STIE = bit 5)
        core::arch::asm!("csrs sie, {}", in(reg) 1usize << 5);
        
        let now = read_time();
        sbi_set_timer(now + INTERVAL);
        
        // Enable Supervisor Interrupts globally (SIE = bit 1)
        core::arch::asm!("csrs sstatus, {}", in(reg) 1usize << 1);
    }
}

pub fn ticks() -> u64 {
    unsafe { core::ptr::read_volatile(&TICKS) }
}

#[no_mangle]
extern "C" fn _trap_rust() {
    let scause: usize;
    unsafe { core::arch::asm!("csrr {}, scause", out(reg) scause) };
    let is_int = (scause >> 63) & 1 == 1;
    let code = scause & 0xff;
    
    // Supervisor Timer Interrupt is code 5
    if is_int && code == 5 {
        unsafe {
            TICKS += 1;
            let now = read_time();
            sbi_set_timer(now + INTERVAL);
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
    "sret",
);
