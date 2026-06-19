use core::ptr;

const PAGE_SIZE: usize = 4096;

struct FreeNode {
    next: *mut FreeNode,
}

static mut FREE_LIST: *mut FreeNode = ptr::null_mut();
static mut FREE_COUNT: usize = 0;

extern "C" {
    static _heap_start: u8;
    static _heap_end: u8;
}

pub fn init() {
    unsafe {
        let start = (&_heap_start as *const u8 as usize + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end = &_heap_end as *const u8 as usize;
        let mut addr = start;
        while addr + PAGE_SIZE <= end {
            free_page(addr as *mut u8);
            addr += PAGE_SIZE;
        }
        crate::println!("mem: {} pages available ({} MB)", FREE_COUNT, FREE_COUNT * 4 / 1024);
    }
}

pub fn alloc_page() -> *mut u8 {
    unsafe {
        if FREE_LIST.is_null() {
            return ptr::null_mut();
        }
        let page = FREE_LIST;
        FREE_LIST = (*page).next;
        FREE_COUNT -= 1;
        ptr::write_bytes(page as *mut u8, 0, PAGE_SIZE);
        page as *mut u8
    }
}

pub fn free_page(p: *mut u8) {
    unsafe {
        let node = p as *mut FreeNode;
        (*node).next = FREE_LIST;
        FREE_LIST = node;
        FREE_COUNT += 1;
    }
}
