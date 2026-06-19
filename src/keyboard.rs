use crate::virtio::{self, VirtqDesc, VIRTQ_DESC_F_WRITE};
use core::sync::atomic::{compiler_fence, Ordering};
use core::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtioInputEvent {
    type_: u16,
    code: u16,
    value: u32,
}

static mut VQ_BLOCK: virtio::VirtQueueBlock = unsafe { core::mem::zeroed() };
static mut KBD_BASE_ADDR: usize = 0;
static mut LAST_USED_IDX: u16 = 0;
static mut EVENTS: [VirtioInputEvent; 16] = [VirtioInputEvent { type_: 0, code: 0, value: 0 }; 16];

pub fn init() -> bool {
    let mut kbd_base = 0;
    unsafe {
        for base in (0x1000_1000..=0x1000_8000).step_by(0x1000) {
            // Device ID 18 = VirtIO Input
            if virtio::init(base, 18, &mut VQ_BLOCK) {
                kbd_base = base;
                break;
            }
        }

        if kbd_base == 0 {
            return false;
        }

        KBD_BASE_ADDR = kbd_base;

        // Populate EventQ (Queue 0) with all 16 buffers
        for i in 0..16 {
            VQ_BLOCK.desc[i] = VirtqDesc {
                addr: &mut EVENTS[i] as *mut _ as u64,
                len: core::mem::size_of::<VirtioInputEvent>() as u32,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            };
            VQ_BLOCK.avail_ring[i] = i as u16;
        }
        
        compiler_fence(Ordering::SeqCst);
        VQ_BLOCK.avail_idx = 16;
        compiler_fence(Ordering::SeqCst);
        
        ptr::write_volatile((kbd_base + 0x050) as *mut u32, 0); // Notify Queue 0
    }
    true
}

// Basic Linux Keycode to ASCII mapping (QWERTY)
fn keycode_to_char(code: u16, shift: bool) -> Option<u8> {
    let unshifted = match code {
        2..=10 => b"123456789"[code as usize - 2],
        11 => b'0', 12 => b'-', 13 => b'=', 14 => 0x08, // Backspace
        15 => b'\t', 16 => b'q', 17 => b'w', 18 => b'e', 19 => b'r', 20 => b't', 21 => b'y', 22 => b'u', 23 => b'i', 24 => b'o', 25 => b'p', 26 => b'[', 27 => b']', 28 => b'\n', // Enter
        30 => b'a', 31 => b's', 32 => b'd', 33 => b'f', 34 => b'g', 35 => b'h', 36 => b'j', 37 => b'k', 38 => b'l', 39 => b';', 40 => b'\'', 41 => b'`',
        43 => b'\\', 44 => b'z', 45 => b'x', 46 => b'c', 47 => b'v', 48 => b'b', 49 => b'n', 50 => b'm', 51 => b',', 52 => b'.', 53 => b'/',
        57 => b' ', // Space
        _ => return None,
    };
    
    if shift {
        let shifted = match unshifted {
            b'a'..=b'z' => unshifted - 32,
            b'1' => b'!', b'2' => b'@', b'3' => b'#', b'4' => b'$', b'5' => b'%', b'6' => b'^', b'7' => b'&', b'8' => b'*', b'9' => b'(', b'0' => b')',
            b'-' => b'_', b'=' => b'+', b'[' => b'{', b']' => b'}', b'\\' => b'|', b';' => b':', b'\'' => b'"', b',' => b'<', b'.' => b'>', b'/' => b'?', b'`' => b'~',
            _ => unshifted,
        };
        Some(shifted)
    } else {
        Some(unshifted)
    }
}

static mut SHIFT_PRESSED: bool = false;

pub fn poll() -> Option<u8> {
    unsafe {
        if KBD_BASE_ADDR == 0 {
            return None;
        }

        if VQ_BLOCK.used_idx == LAST_USED_IDX {
            return None;
        }

        let used_idx = LAST_USED_IDX % 16;
        let desc_id = VQ_BLOCK.used_ring[used_idx as usize].id as usize;
        let event = EVENTS[desc_id];

        // Re-queue the descriptor immediately
        let avail_idx = VQ_BLOCK.avail_idx;
        VQ_BLOCK.avail_ring[(avail_idx % 16) as usize] = desc_id as u16;
        compiler_fence(Ordering::SeqCst);
        VQ_BLOCK.avail_idx = avail_idx.wrapping_add(1);
        compiler_fence(Ordering::SeqCst);
        ptr::write_volatile((KBD_BASE_ADDR + 0x050) as *mut u32, 0); // Notify Queue 0

        LAST_USED_IDX = LAST_USED_IDX.wrapping_add(1);

        // EV_KEY
        if event.type_ == 1 {
            if event.code == 42 || event.code == 54 { // LSHIFT or RSHIFT
                SHIFT_PRESSED = event.value != 0;
                return None;
            }

            // If pressed or repeated
            if event.value == 1 || event.value == 2 {
                return keycode_to_char(event.code, SHIFT_PRESSED);
            }
        }
        
        None
    }
}
