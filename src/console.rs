use core::fmt;

struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                crate::uart::putc(b'\r');
            }
            crate::uart::putc(b);
        }
        crate::gpu::print_str(s);
        crate::gpu::flush();
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    Writer.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn read_line(buf: &mut [u8]) -> usize {
    let mut i = 0;
    loop {
        let c = match crate::uart::getc() {
            Some(c) => c,
            None => match crate::keyboard::poll() {
                Some(k) => k,
                None => continue,
            }
        };
        match c {
            b'\r' | b'\n' => {
                print!("\r\n");
                return i;
            }
            0x7f | 0x08 => {
                if i > 0 {
                    i -= 1;
                    print!("\x08 \x08");
                }
            }
            0x03 => return 0, // Ctrl-C
            c if c >= 0x20 && i < buf.len() - 1 => {
                buf[i] = c;
                i += 1;
                print!("{}", c as char);
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Key {
    Char(u8),
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Delete,
    Backspace,
    Enter,
    CtrlS,
    CtrlQ,
}

fn wait_byte() -> Option<u8> {
    for _ in 0..100_000 {
        if let Some(c) = crate::uart::getc() {
            return Some(c);
        }
        if let Some(k) = crate::keyboard::poll() {
            return Some(k);
        }
    }
    None
}

pub fn read_key() -> Key {
    let c = loop {
        if let Some(c) = crate::uart::getc() {
            break c;
        }
        if let Some(k) = crate::keyboard::poll() {
            break k;
        }
    };
    match c {
        0x1b => match wait_byte() {
            Some(b'[') => match wait_byte() {
                Some(b'A') => Key::Up,
                Some(b'B') => Key::Down,
                Some(b'C') => Key::Right,
                Some(b'D') => Key::Left,
                Some(b'H') => Key::Home,
                Some(b'F') => Key::End,
                Some(b'3') => {
                    wait_byte();
                    Key::Delete
                }
                _ => Key::Char(0x1b),
            },
            _ => Key::Char(0x1b),
        },
        0x13 => Key::CtrlS,
        0x11 => Key::CtrlQ,
        0x0d => Key::Enter,
        0x7f | 0x08 => Key::Backspace,
        c if c >= 0x20 => Key::Char(c),
        _ => Key::Char(c),
    }
}

pub fn clear_screen() {
    print!("\x1b[2J\x1b[H");
}

pub fn move_cursor(row: usize, col: usize) {
    print!("\x1b[{};{}H", row + 1, col + 1);
}
