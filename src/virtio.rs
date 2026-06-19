use core::sync::atomic::{compiler_fence, Ordering};
use core::ptr;

// VirtIO MMIO Offsets (Legacy Version 1)
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000;
const VIRTIO_MMIO_VERSION: usize = 0x004;
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;
const VIRTIO_MMIO_STATUS: usize = 0x070;
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x028;

#[repr(C, align(4096))]
pub struct VirtQueueBlock {
    // Descriptors
    pub desc: [VirtqDesc; 16],
    // Avail Ring
    pub avail_flags: u16,
    pub avail_idx: u16,
    pub avail_ring: [u16; 16],
    pub avail_used_event: u16,
    // Padding to page boundary
    pub pad: [u8; 4096 - (16 * 16 + 4 + 16 * 2 + 2)],
    // Used Ring
    pub used_flags: u16,
    pub used_idx: u16,
    pub used_ring: [VirtqUsedElem; 16],
    pub used_avail_event: u16,
}

#[repr(C)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;

pub fn init(base: usize, expected_device_id: u32, vq_block: &mut VirtQueueBlock) -> bool {
    unsafe {
        let magic = ptr::read_volatile((base + VIRTIO_MMIO_MAGIC_VALUE) as *const u32);
        let version = ptr::read_volatile((base + VIRTIO_MMIO_VERSION) as *const u32);
        let device_id = ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_ID) as *const u32);
        
        if magic != 0x74726976 || version != 1 || device_id != expected_device_id {
            return false;
        }

        ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 0); // Reset
        ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 1); // Acknowledge
        ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 1 | 2); // Driver

        // Select Queue 0
        ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_SEL) as *mut u32, 0);
        let max_num = ptr::read_volatile((base + VIRTIO_MMIO_QUEUE_NUM_MAX) as *const u32);
        if max_num < 16 { return false; }
        
        ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_NUM) as *mut u32, 16);
        ptr::write_volatile((base + VIRTIO_MMIO_GUEST_PAGE_SIZE) as *mut u32, 4096);
        
        let pfn = (vq_block as *const VirtQueueBlock as usize) / 4096;
        ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_PFN) as *mut u32, pfn as u32);
        
        ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 1 | 2 | 4); // Driver OK
    }
    true
}

// Simple synchronous send with timeout
pub fn send_command(base: usize, vq_block: &mut VirtQueueBlock, last_used_idx: &mut u16, setup_desc: impl FnOnce(&mut [VirtqDesc; 16])) -> bool {
    unsafe {
        setup_desc(&mut vq_block.desc);
        
        compiler_fence(Ordering::SeqCst);
        let head = 0; // Always use index 0 as head
        let avail_idx = vq_block.avail_idx;
        vq_block.avail_ring[(avail_idx % 16) as usize] = head;
        compiler_fence(Ordering::SeqCst);
        vq_block.avail_idx = avail_idx.wrapping_add(1);
        compiler_fence(Ordering::SeqCst);
        
        ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_NOTIFY) as *mut u32, 0);
        
        // Wait for device to process with timeout
        let start_ticks = crate::trap::ticks();
        while vq_block.used_idx == *last_used_idx {
            if crate::trap::ticks() - start_ticks > 50 { // ~500ms timeout
                return false;
            }
            core::arch::asm!("nop");
            compiler_fence(Ordering::SeqCst);
        }
        *last_used_idx = vq_block.used_idx;
        true
    }
}
