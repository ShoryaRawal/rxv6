use crate::virtio::{self, VirtqDesc, VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE};
use core::ptr;

const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0100;
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
const VIRTIO_GPU_CMD_RESOURCE_UNREF: u32 = 0x0102;
const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;

#[repr(C)]
struct VirtioGpuCtrlHdr {
    type_: u32,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    padding: u32,
}

#[repr(C)]
struct VirtioGpuResourceCreate2d {
    hdr: VirtioGpuCtrlHdr,
    resource_id: u32,
    format: u32, // 1 = B8G8R8A8_UNORM
    width: u32,
    height: u32,
}

#[repr(C)]
struct VirtioGpuResourceAttachBacking {
    hdr: VirtioGpuCtrlHdr,
    resource_id: u32,
    nr_entries: u32,
}

#[repr(C)]
struct VirtioGpuMemEntry {
    addr: u64,
    length: u32,
    padding: u32,
}

#[repr(C)]
struct VirtioGpuSetScanout {
    hdr: VirtioGpuCtrlHdr,
    r_x: u32,
    r_y: u32,
    r_width: u32,
    r_height: u32,
    scanout_id: u32,
    resource_id: u32,
}

#[repr(C)]
struct VirtioGpuTransferToHost2d {
    hdr: VirtioGpuCtrlHdr,
    r_x: u32,
    r_y: u32,
    r_width: u32,
    r_height: u32,
    offset: u64,
    resource_id: u32,
    padding: u32,
}

#[repr(C)]
struct VirtioGpuResourceFlush {
    hdr: VirtioGpuCtrlHdr,
    r_x: u32,
    r_y: u32,
    r_width: u32,
    r_height: u32,
    resource_id: u32,
    padding: u32,
}

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;

static mut FRAMEBUFFER: [u32; (WIDTH * HEIGHT) as usize] = [0; (WIDTH * HEIGHT) as usize];
static mut GPU_BASE_ADDR: usize = 0;

static mut VQ_BLOCK: crate::virtio::VirtQueueBlock = unsafe { core::mem::zeroed() };
static mut LAST_USED_IDX: u16 = 0;

pub fn init() -> bool {
    let mut gpu_base = 0;
    unsafe {
        for base in (0x1000_1000..=0x1000_8000).step_by(0x1000) {
            if virtio::init(base, 16, &mut VQ_BLOCK) {
                gpu_base = base;
                break;
            }
        }
    }

    if gpu_base == 0 {
        return false;
    }

    unsafe {
        GPU_BASE_ADDR = gpu_base;
        let mut create = VirtioGpuResourceCreate2d {
            hdr: VirtioGpuCtrlHdr { type_: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D, flags: 0, fence_id: 0, ctx_id: 0, padding: 0 },
            resource_id: 1,
            format: 1,
            width: WIDTH,
            height: HEIGHT,
        };
        let mut resp = [0u8; 24];
        if !virtio::send_command(GPU_BASE_ADDR, &mut VQ_BLOCK, &mut LAST_USED_IDX, |desc| {
            desc[0] = VirtqDesc { addr: &create as *const _ as u64, len: core::mem::size_of_val(&create) as u32, flags: VIRTQ_DESC_F_NEXT, next: 1 };
            desc[1] = VirtqDesc { addr: &mut resp as *mut _ as u64, len: 24, flags: VIRTQ_DESC_F_WRITE, next: 0 };
        }) { return false; }

        let mut attach = VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHdr { type_: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING, flags: 0, fence_id: 0, ctx_id: 0, padding: 0 },
            resource_id: 1,
            nr_entries: 1,
        };
        let mem_entry = VirtioGpuMemEntry {
            addr: FRAMEBUFFER.as_ptr() as u64,
            length: (WIDTH * HEIGHT * 4) as u32,
            padding: 0,
        };
        if !virtio::send_command(GPU_BASE_ADDR, &mut VQ_BLOCK, &mut LAST_USED_IDX, |desc| {
            desc[0] = VirtqDesc { addr: &attach as *const _ as u64, len: core::mem::size_of_val(&attach) as u32, flags: VIRTQ_DESC_F_NEXT, next: 1 };
            desc[1] = VirtqDesc { addr: &mem_entry as *const _ as u64, len: core::mem::size_of_val(&mem_entry) as u32, flags: VIRTQ_DESC_F_NEXT, next: 2 };
            desc[2] = VirtqDesc { addr: &mut resp as *mut _ as u64, len: 24, flags: VIRTQ_DESC_F_WRITE, next: 0 };
        }) { return false; }

        let mut scanout = VirtioGpuSetScanout {
            hdr: VirtioGpuCtrlHdr { type_: VIRTIO_GPU_CMD_SET_SCANOUT, flags: 0, fence_id: 0, ctx_id: 0, padding: 0 },
            r_x: 0, r_y: 0, r_width: WIDTH, r_height: HEIGHT,
            scanout_id: 0, resource_id: 1,
        };
        if !virtio::send_command(GPU_BASE_ADDR, &mut VQ_BLOCK, &mut LAST_USED_IDX, |desc| {
            desc[0] = VirtqDesc { addr: &scanout as *const _ as u64, len: core::mem::size_of_val(&scanout) as u32, flags: VIRTQ_DESC_F_NEXT, next: 1 };
            desc[1] = VirtqDesc { addr: &mut resp as *mut _ as u64, len: 24, flags: VIRTQ_DESC_F_WRITE, next: 0 };
        }) { return false; }
    }

    true
}

pub fn flush() {
    unsafe {
        if GPU_BASE_ADDR == 0 {
            return;
        }
        let mut transfer = VirtioGpuTransferToHost2d {
            hdr: VirtioGpuCtrlHdr { type_: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D, flags: 0, fence_id: 0, ctx_id: 0, padding: 0 },
            r_x: 0, r_y: 0, r_width: WIDTH, r_height: HEIGHT,
            offset: 0, resource_id: 1, padding: 0,
        };
        let mut resp = [0u8; 24];
        let _ = virtio::send_command(GPU_BASE_ADDR, &mut VQ_BLOCK, &mut LAST_USED_IDX, |desc| {
            desc[0] = VirtqDesc { addr: &transfer as *const _ as u64, len: core::mem::size_of_val(&transfer) as u32, flags: VIRTQ_DESC_F_NEXT, next: 1 };
            desc[1] = VirtqDesc { addr: &mut resp as *mut _ as u64, len: 24, flags: VIRTQ_DESC_F_WRITE, next: 0 };
        });

        let mut flush = VirtioGpuResourceFlush {
            hdr: VirtioGpuCtrlHdr { type_: VIRTIO_GPU_CMD_RESOURCE_FLUSH, flags: 0, fence_id: 0, ctx_id: 0, padding: 0 },
            r_x: 0, r_y: 0, r_width: WIDTH, r_height: HEIGHT,
            resource_id: 1, padding: 0,
        };
        let _ = virtio::send_command(GPU_BASE_ADDR, &mut VQ_BLOCK, &mut LAST_USED_IDX, |desc| {
            desc[0] = VirtqDesc { addr: &flush as *const _ as u64, len: core::mem::size_of_val(&flush) as u32, flags: VIRTQ_DESC_F_NEXT, next: 1 };
            desc[1] = VirtqDesc { addr: &mut resp as *mut _ as u64, len: 24, flags: VIRTQ_DESC_F_WRITE, next: 0 };
        });
    }
}

pub fn put_pixel(x: u32, y: u32, color: u32) {
    if x < WIDTH && y < HEIGHT {
        unsafe {
            FRAMEBUFFER[(y * WIDTH + x) as usize] = color;
        }
    }
}

pub fn draw_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    for i in 0..w {
        for j in 0..h {
            put_pixel(x + i, y + j, color);
        }
    }
}

pub static mut CURSOR_X: u32 = 0;
pub static mut CURSOR_Y: u32 = 0;
const CHAR_WIDTH: u32 = 8;
const CHAR_HEIGHT: u32 = 8;
const SCALE: u32 = 2;

pub fn draw_char(c: char, fg: u32, bg: u32) {
    unsafe {
        if c == '\n' {
            CURSOR_X = 0;
            CURSOR_Y += CHAR_HEIGHT * SCALE;
            check_scroll();
            return;
        } else if c == '\r' {
            CURSOR_X = 0;
            return;
        } else if c == '\x08' {
            if CURSOR_X >= CHAR_WIDTH * SCALE {
                CURSOR_X -= CHAR_WIDTH * SCALE;
            } else if CURSOR_Y >= CHAR_HEIGHT * SCALE {
                CURSOR_Y -= CHAR_HEIGHT * SCALE;
                CURSOR_X = WIDTH - (CHAR_WIDTH * SCALE);
            }
            draw_rect(CURSOR_X, CURSOR_Y, CHAR_WIDTH * SCALE, CHAR_HEIGHT * SCALE, bg);
            return;
        }

        let idx = c as usize;
        if idx >= 128 { return; }

        let bitmap = crate::font::FONT8X8[idx];

        for row in 0..8 {
            for col in 0..8 {
                let color = if (bitmap[row] & (1 << col)) != 0 { fg } else { bg };
                let px = CURSOR_X + (col as u32) * SCALE;
                let py = CURSOR_Y + (row as u32) * SCALE;
                put_pixel(px, py, color);
                put_pixel(px + 1, py, color);
                put_pixel(px, py + 1, color);
                put_pixel(px + 1, py + 1, color);
            }
        }

        CURSOR_X += CHAR_WIDTH * SCALE;
        if CURSOR_X >= WIDTH {
            CURSOR_X = 0;
            CURSOR_Y += CHAR_HEIGHT * SCALE;
            check_scroll();
        }
    }
}

fn check_scroll() {
    unsafe {
        if CURSOR_Y >= HEIGHT {
            // Scroll up by one line
            let line_bytes = (WIDTH * CHAR_HEIGHT * SCALE) as usize;
            for i in line_bytes..FRAMEBUFFER.len() {
                FRAMEBUFFER[i - line_bytes] = FRAMEBUFFER[i];
            }
            // Clear last line
            for i in (FRAMEBUFFER.len() - line_bytes)..FRAMEBUFFER.len() {
                FRAMEBUFFER[i] = 0xFF0000FF; // Blue background
            }
            CURSOR_Y -= CHAR_HEIGHT * SCALE;
        }
    }
}

static mut ANSI_STATE: u8 = 0;
static mut ANSI_PARAM: u32 = 0;
static mut ANSI_PARAM2: u32 = 0;

pub fn print_str(s: &str) {
    unsafe {
        for c in s.chars() {
            if ANSI_STATE == 0 {
                if c == '\x1b' {
                    ANSI_STATE = 1;
                } else {
                    draw_char(c, 0xFFFFFFFF, 0xFF0000FF);
                }
            } else if ANSI_STATE == 1 {
                if c == '[' {
                    ANSI_STATE = 2;
                    ANSI_PARAM = 0;
                    ANSI_PARAM2 = 0;
                } else {
                    ANSI_STATE = 0;
                }
            } else if ANSI_STATE == 2 {
                if c >= '0' && c <= '9' {
                    ANSI_PARAM = ANSI_PARAM * 10 + (c as u32 - '0' as u32);
                } else if c == ';' {
                    ANSI_STATE = 3;
                } else if c == 'J' {
                    if ANSI_PARAM == 2 {
                        draw_rect(0, 0, WIDTH, HEIGHT, 0xFF0000FF);
                        CURSOR_X = 0;
                        CURSOR_Y = 0;
                    }
                    ANSI_STATE = 0;
                } else if c == 'H' {
                    let row = if ANSI_PARAM > 0 { ANSI_PARAM - 1 } else { 0 };
                    let col = 0; // if only one param, col is 0
                    CURSOR_Y = row * CHAR_HEIGHT * SCALE;
                    CURSOR_X = col * CHAR_WIDTH * SCALE;
                    ANSI_STATE = 0;
                } else {
                    ANSI_STATE = 0;
                }
            } else if ANSI_STATE == 3 {
                if c >= '0' && c <= '9' {
                    ANSI_PARAM2 = ANSI_PARAM2 * 10 + (c as u32 - '0' as u32);
                } else if c == 'H' {
                    let row = if ANSI_PARAM > 0 { ANSI_PARAM - 1 } else { 0 };
                    let col = if ANSI_PARAM2 > 0 { ANSI_PARAM2 - 1 } else { 0 };
                    CURSOR_Y = row * CHAR_HEIGHT * SCALE;
                    CURSOR_X = col * CHAR_WIDTH * SCALE;
                    ANSI_STATE = 0;
                } else {
                    ANSI_STATE = 0;
                }
            }
        }
    }
}
