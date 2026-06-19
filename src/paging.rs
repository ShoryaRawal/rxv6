use core::ptr;

// Sv39 Page Table Entry (PTE) flags
const PTE_V: u64 = 1 << 0; // Valid
const PTE_R: u64 = 1 << 1; // Read
const PTE_W: u64 = 1 << 2; // Write
const PTE_X: u64 = 1 << 3; // Execute
const PTE_U: u64 = 1 << 4; // User
const PTE_A: u64 = 1 << 6; // Accessed
const PTE_D: u64 = 1 << 7; // Dirty

// Physical memory constants
const RAM_START: usize = 0x8000_0000;

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u64; 512],
}

static mut ROOT_PT: PageTable = PageTable { entries: [0; 512] };

pub fn init() {
    unsafe {
        // Identity map RAM (Read, Write, Execute) using 1GB megapage (level 2)
        // VPN[2] for 0x80000000 is 2.
        let pte = (RAM_START as u64 >> 12) << 10 | PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;
        ROOT_PT.entries[2] = pte;

        // Identity map UART, VirtIO, CLINT using 1GB megapage (level 2)
        // VPN[2] for 0x00000000 is 0. This covers up to 0x3FFFFFFF.
        let pte_io = (0 as u64 >> 12) << 10 | PTE_V | PTE_R | PTE_W | PTE_A | PTE_D;
        ROOT_PT.entries[0] = pte_io;

        // Enable paging (Sv39 mode = 8)
        let root_ppn = (&ROOT_PT as *const PageTable as usize >> 12) as u64;
        let satp = (8 << 60) | root_ppn;
        
        core::arch::asm!("csrw satp, {}", in(reg) satp);
        core::arch::asm!("sfence.vma zero, zero");
    }
}
