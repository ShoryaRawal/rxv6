const BASE: usize = 0x1000_0000;

const THR: usize = 0x00;
const RBR: usize = 0x00;
const IER: usize = 0x01;
const FCR: usize = 0x02;
const LCR: usize = 0x03;
const LSR: usize = 0x05;

unsafe fn write_reg(off: usize, val: u8) {
    ((BASE + off) as *mut u8).write_volatile(val);
}

unsafe fn read_reg(off: usize) -> u8 {
    ((BASE + off) as *const u8).read_volatile()
}

pub fn init() {
    unsafe {
        write_reg(IER, 0x00);
        write_reg(LCR, 0x80);
        write_reg(THR, 0x01);
        write_reg(IER, 0x00);
        write_reg(LCR, 0x03);
        write_reg(FCR, 0x07);
        write_reg(IER, 0x01);
    }
}

pub fn putc(c: u8) {
    unsafe {
        while read_reg(LSR) & 0x20 == 0 {}
        write_reg(THR, c);
    }
}

pub fn getc() -> Option<u8> {
    unsafe {
        if read_reg(LSR) & 0x01 != 0 {
            Some(read_reg(RBR))
        } else {
            None
        }
    }
}
