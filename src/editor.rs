use crate::console::Key;

const ROWS: usize = 22;
const MAX_LINES: usize = 64;
const MAX_LINE_LEN: usize = 80;

static mut LINES: [[u8; MAX_LINE_LEN]; MAX_LINES] = [[0; MAX_LINE_LEN]; MAX_LINES];
static mut LENS: [usize; MAX_LINES] = [0; MAX_LINES];
static mut NLINES: usize = 1;
static mut CUR_R: usize = 0;
static mut CUR_C: usize = 0;
static mut OFFSET: usize = 0;
static mut DIRTY: bool = false;
static mut FNAME: [u8; 64] = [0; 64];
static mut FNAME_LEN: usize = 0;
static mut STATUS: [u8; 64] = [0; 64];
static mut STATUS_LEN: usize = 0;

fn set_status(msg: &str) {
    unsafe {
        let len = msg.len().min(64);
        STATUS[..len].copy_from_slice(&msg.as_bytes()[..len]);
        STATUS_LEN = len;
    }
}

fn load_content(data: &[u8]) {
    unsafe {
        NLINES = 0;
        LINES = [[0; MAX_LINE_LEN]; MAX_LINES];
        LENS = [0; MAX_LINES];
        let mut start = 0;
        for i in 0..data.len() {
            if data[i] == b'\n' {
                let len = (i - start).min(MAX_LINE_LEN);
                LINES[NLINES][..len].copy_from_slice(&data[start..start + len]);
                LENS[NLINES] = len;
                NLINES += 1;
                start = i + 1;
                if NLINES >= MAX_LINES {
                    return;
                }
            }
        }
        if start < data.len() {
            let len = (data.len() - start).min(MAX_LINE_LEN);
            LINES[NLINES][..len].copy_from_slice(&data[start..start + len]);
            LENS[NLINES] = len;
            NLINES += 1;
        }
        if NLINES == 0 {
            NLINES = 1;
        }
    }
}

fn save() {
    unsafe {
        let mut buf = [0u8; crate::fs::MAX_FILE_SIZE];
        let mut pos = 0;
        for i in 0..NLINES {
            let len = LENS[i];
            if pos + len + 1 > buf.len() {
                set_status("File too large!");
                return;
            }
            buf[pos..pos + len].copy_from_slice(&LINES[i][..len]);
            pos += len;
            buf[pos] = b'\n';
            pos += 1;
        }
        let name = core::str::from_utf8(&FNAME[..FNAME_LEN]).unwrap_or("");
        if !crate::fs::exists(name) {
            let _ = crate::fs::create_file(name);
        }
        match crate::fs::write_file(name, &buf[..pos]) {
            Ok(()) => {
                DIRTY = false;
                set_status("Saved.");
            }
            Err(e) => {
                let mut tmp = [0u8; 64];
                let msg = format_err(&mut tmp, &e);
                set_status(msg);
            }
        }
    }
}

fn format_err<'a>(buf: &'a mut [u8; 64], e: &crate::fs::FsError) -> &'a str {
    use core::fmt::Write;
    struct BufWriter<'b> { buf: &'b mut [u8], pos: usize }
    impl<'b> Write for BufWriter<'b> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let len = bytes.len().min(self.buf.len() - self.pos);
            self.buf[self.pos..self.pos + len].copy_from_slice(&bytes[..len]);
            self.pos += len;
            Ok(())
        }
    }
    let mut w = BufWriter { buf, pos: 0 };
    let _ = write!(w, "Error: {}", e);
    let len = w.pos;
    core::str::from_utf8(&buf[..len]).unwrap_or("error")
}

fn scroll() {
    unsafe {
        if CUR_R < OFFSET {
            OFFSET = CUR_R;
        }
        if CUR_R >= OFFSET + ROWS {
            OFFSET = CUR_R - ROWS + 1;
        }
    }
}

fn draw() {
    unsafe {
        scroll();
        crate::print!("\x1b[H");
        for sr in 0..ROWS {
            let fr = OFFSET + sr;
            crate::print!("\x1b[K");
            if fr < NLINES {
                let len = LENS[fr].min(MAX_LINE_LEN);
                if let Ok(s) = core::str::from_utf8(&LINES[fr][..len]) {
                    crate::print!("{}", s);
                }
            } else {
                crate::print!("~");
            }
            crate::print!("\r\n");
        }
        crate::print!("\x1b[7m\x1b[K");
        let name = core::str::from_utf8(&FNAME[..FNAME_LEN]).unwrap_or("new");
        let marker = if DIRTY { " [+]" } else { "" };
        crate::print!(" {}{} | L{}/{}", name, marker, CUR_R + 1, NLINES);
        if STATUS_LEN > 0 {
            let msg = core::str::from_utf8(&STATUS[..STATUS_LEN]).unwrap_or("");
            crate::print!("  {}", msg);
        }
        crate::print!("\x1b[0m\r\n");
        crate::print!("\x1b[K ^S Save  ^Q Quit");
        let sy = CUR_R - OFFSET + 1;
        let sx = CUR_C + 1;
        crate::print!("\x1b[{};{}H", sy, sx);
    }
}

fn insert_char(c: u8) {
    unsafe {
        let len = LENS[CUR_R];
        if len >= MAX_LINE_LEN - 1 {
            return;
        }
        let mut i = len;
        while i > CUR_C {
            LINES[CUR_R][i] = LINES[CUR_R][i - 1];
            i -= 1;
        }
        LINES[CUR_R][CUR_C] = c;
        LENS[CUR_R] += 1;
        CUR_C += 1;
        DIRTY = true;
    }
}

fn insert_newline() {
    unsafe {
        if NLINES >= MAX_LINES {
            return;
        }
        let mut i = NLINES;
        while i > CUR_R + 1 {
            LINES[i] = LINES[i - 1];
            LENS[i] = LENS[i - 1];
            i -= 1;
        }
        let old_len = LENS[CUR_R];
        let new_len = old_len - CUR_C;
        LINES[CUR_R + 1] = [0; MAX_LINE_LEN];
        LINES[CUR_R + 1][..new_len].copy_from_slice(&LINES[CUR_R][CUR_C..old_len]);
        LENS[CUR_R + 1] = new_len;
        for j in CUR_C..old_len {
            LINES[CUR_R][j] = 0;
        }
        LENS[CUR_R] = CUR_C;
        NLINES += 1;
        CUR_R += 1;
        CUR_C = 0;
        DIRTY = true;
    }
}

fn delete_back() {
    unsafe {
        if CUR_C > 0 {
            let len = LENS[CUR_R];
            for i in (CUR_C - 1)..len.saturating_sub(1) {
                LINES[CUR_R][i] = LINES[CUR_R][i + 1];
            }
            if len > 0 {
                LINES[CUR_R][len - 1] = 0;
                LENS[CUR_R] -= 1;
            }
            CUR_C -= 1;
            DIRTY = true;
        } else if CUR_R > 0 {
            let prev_len = LENS[CUR_R - 1];
            let cur_len = LENS[CUR_R];
            if prev_len + cur_len <= MAX_LINE_LEN {
                LINES[CUR_R - 1][prev_len..prev_len + cur_len]
                    .copy_from_slice(&LINES[CUR_R][..cur_len]);
                LENS[CUR_R - 1] = prev_len + cur_len;
                for i in CUR_R..NLINES - 1 {
                    LINES[i] = LINES[i + 1];
                    LENS[i] = LENS[i + 1];
                }
                NLINES -= 1;
                LINES[NLINES] = [0; MAX_LINE_LEN];
                LENS[NLINES] = 0;
                CUR_R -= 1;
                CUR_C = prev_len;
                DIRTY = true;
            }
        }
    }
}

fn delete_forward() {
    unsafe {
        let len = LENS[CUR_R];
        if CUR_C < len {
            for i in CUR_C..len - 1 {
                LINES[CUR_R][i] = LINES[CUR_R][i + 1];
            }
            LINES[CUR_R][len - 1] = 0;
            LENS[CUR_R] -= 1;
            DIRTY = true;
        } else if CUR_R < NLINES - 1 {
            let next_len = LENS[CUR_R + 1];
            if len + next_len <= MAX_LINE_LEN {
                LINES[CUR_R][len..len + next_len]
                    .copy_from_slice(&LINES[CUR_R + 1][..next_len]);
                LENS[CUR_R] = len + next_len;
                for i in (CUR_R + 1)..NLINES - 1 {
                    LINES[i] = LINES[i + 1];
                    LENS[i] = LENS[i + 1];
                }
                NLINES -= 1;
                LINES[NLINES] = [0; MAX_LINE_LEN];
                LENS[NLINES] = 0;
                DIRTY = true;
            }
        }
    }
}

pub fn edit(filename: &str) {
    if filename.is_empty() {
        crate::println!("usage: edit <filename>");
        return;
    }
    unsafe {
        NLINES = 1;
        LINES = [[0; MAX_LINE_LEN]; MAX_LINES];
        LENS = [0; MAX_LINES];
        CUR_R = 0;
        CUR_C = 0;
        OFFSET = 0;
        DIRTY = false;
        STATUS = [0; 64];
        STATUS_LEN = 0;
        let nb = filename.as_bytes();
        let len = nb.len().min(64);
        FNAME = [0; 64];
        FNAME[..len].copy_from_slice(&nb[..len]);
        FNAME_LEN = len;
    }
    if crate::fs::exists(filename) && !crate::fs::is_directory(filename) {
        let mut data = [0u8; crate::fs::MAX_FILE_SIZE];
        if let Ok(n) = crate::fs::read_file(filename, &mut data) {
            load_content(&data[..n]);
        }
    }
    crate::console::clear_screen();
    draw();
    loop {
        unsafe { STATUS_LEN = 0; }
        let key = crate::console::read_key();
        match key {
            Key::CtrlQ => break,
            Key::CtrlS => save(),
            Key::Up => unsafe {
                if CUR_R > 0 {
                    CUR_R -= 1;
                    CUR_C = CUR_C.min(LENS[CUR_R]);
                }
            },
            Key::Down => unsafe {
                if CUR_R < NLINES - 1 {
                    CUR_R += 1;
                    CUR_C = CUR_C.min(LENS[CUR_R]);
                }
            },
            Key::Left => unsafe {
                if CUR_C > 0 {
                    CUR_C -= 1;
                }
            },
            Key::Right => unsafe {
                if CUR_C < LENS[CUR_R] {
                    CUR_C += 1;
                }
            },
            Key::Home => unsafe { CUR_C = 0; },
            Key::End => unsafe { CUR_C = LENS[CUR_R]; },
            Key::Enter => insert_newline(),
            Key::Backspace => delete_back(),
            Key::Delete => delete_forward(),
            Key::Char(c) if c >= 0x20 && c < 0x7f => insert_char(c),
            _ => {}
        }
        draw();
    }
    crate::console::clear_screen();
}
