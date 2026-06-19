use crate::fs::DirEntry;

pub fn run() -> ! {
    loop {
        let mut pbuf = [0u8; 256];
        let plen = crate::fs::cwd_path(&mut pbuf);
        let path = core::str::from_utf8(&pbuf[..plen]).unwrap_or("/");
        crate::print!("[{}]$ ", path);

        let mut buf = [0u8; 256];
        let len = crate::console::read_line(&mut buf);
        let line = match core::str::from_utf8(&buf[..len]) {
            Ok(s) => s.trim(),
            Err(_) => continue,
        };
        if line.is_empty() {
            continue;
        }
        let (cmd, args) = match line.find(' ') {
            Some(i) => (&line[..i], line[i + 1..].trim()),
            None => (line, ""),
        };
        match cmd {
            "help" => cmd_help(),
            "ls" => cmd_ls(args),
            "cd" => cmd_cd(args),
            "pwd" => cmd_pwd(),
            "cat" => cmd_cat(args),
            "echo" => cmd_echo(args),
            "touch" => cmd_touch(args),
            "mkdir" => cmd_mkdir(args),
            "rm" => cmd_rm(args),
            "mv" => cmd_mv(args),
            "clear" => crate::console::clear_screen(),
            "edit" => crate::editor::edit(args),
            "uname" => crate::println!("RxV6 0.1.0 riscv64"),
            "uptime" => {
                let t = crate::trap::ticks();
                crate::println!("up {}h {}m {}s", t / 3600, (t / 60) % 60, t % 60);
            }
            "shutdown" => {
                crate::println!("Powering off...");
                unsafe { (0x100000 as *mut u32).write_volatile(0x5555); }
                loop {}
            }
            _ => crate::println!("rxv6: {}: command not found", cmd),
        }
    }
}

fn cmd_help() {
    crate::println!("Available commands:");
    crate::println!("  ls [path]        List directory");
    crate::println!("  cd <path>        Change directory");
    crate::println!("  pwd              Working directory");
    crate::println!("  cat <file>       Print file");
    crate::println!("  echo <text>      Print text (> file to redirect)");
    crate::println!("  touch <file>     Create file");
    crate::println!("  mkdir <dir>      Create directory");
    crate::println!("  rm <path>        Remove file/dir");
    crate::println!("  mv <src> <dst>   Move/rename");
    crate::println!("  edit <file>      Text editor");
    crate::println!("  clear            Clear screen");
    crate::println!("  uname            System info");
    crate::println!("  uptime           Uptime");
    crate::println!("  shutdown         Power off");
}

fn cmd_ls(args: &str) {
    let path = if args.is_empty() { "." } else { args };
    let mut entries = [DirEntry {
        name: [0; 28],
        name_len: 0,
        is_dir: false,
        size: 0,
    }; 32];
    match crate::fs::list_dir(path, &mut entries) {
        Ok(count) => {
            for i in 0..count {
                let e = &entries[i];
                let name = core::str::from_utf8(&e.name[..e.name_len]).unwrap_or("?");
                if e.is_dir {
                    crate::println!("{}/ ", name);
                } else {
                    crate::println!("{}  ({} bytes)", name, e.size);
                }
            }
        }
        Err(e) => crate::println!("ls: {}", e),
    }
}

fn cmd_cd(args: &str) {
    let path = if args.is_empty() { "/" } else { args };
    if let Err(e) = crate::fs::set_cwd(path) {
        crate::println!("cd: {}", e);
    }
}

fn cmd_pwd() {
    let mut buf = [0u8; 256];
    let len = crate::fs::cwd_path(&mut buf);
    let path = core::str::from_utf8(&buf[..len]).unwrap_or("/");
    crate::println!("{}", path);
}

fn cmd_cat(args: &str) {
    if args.is_empty() {
        crate::println!("cat: missing filename");
        return;
    }
    let mut buf = [0u8; 4096];
    match crate::fs::read_file(args, &mut buf) {
        Ok(n) => {
            if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                crate::print!("{}", s);
            }
        }
        Err(e) => crate::println!("cat: {}", e),
    }
}

fn cmd_echo(args: &str) {
    if let Some(pos) = args.find('>') {
        let text = args[..pos].trim();
        let file = args[pos + 1..].trim();
        if file.is_empty() {
            crate::println!("echo: missing filename");
            return;
        }
        let mut data = [0u8; 4096];
        let tb = text.as_bytes();
        let len = tb.len().min(4095);
        data[..len].copy_from_slice(&tb[..len]);
        data[len] = b'\n';
        if !crate::fs::exists(file) {
            if let Err(e) = crate::fs::create_file(file) {
                crate::println!("echo: {}", e);
                return;
            }
        }
        if let Err(e) = crate::fs::write_file(file, &data[..len + 1]) {
            crate::println!("echo: {}", e);
        }
    } else {
        crate::println!("{}", args);
    }
}

fn cmd_touch(args: &str) {
    if args.is_empty() {
        crate::println!("touch: missing filename");
        return;
    }
    if !crate::fs::exists(args) {
        if let Err(e) = crate::fs::create_file(args) {
            crate::println!("touch: {}", e);
        }
    }
}

fn cmd_mkdir(args: &str) {
    if args.is_empty() {
        crate::println!("mkdir: missing name");
        return;
    }
    if let Err(e) = crate::fs::create_dir(args) {
        crate::println!("mkdir: {}", e);
    }
}

fn cmd_rm(args: &str) {
    if args.is_empty() {
        crate::println!("rm: missing path");
        return;
    }
    if let Err(e) = crate::fs::delete(args) {
        crate::println!("rm: {}", e);
    }
}

fn cmd_mv(args: &str) {
    let mut parts = args.splitn(2, ' ');
    let src = match parts.next() {
        Some(s) if !s.is_empty() => s,
        _ => {
            crate::println!("mv: missing source");
            return;
        }
    };
    let dst = match parts.next() {
        Some(s) if !s.is_empty() => s.trim(),
        _ => {
            crate::println!("mv: missing destination");
            return;
        }
    };
    if let Err(e) = crate::fs::rename(src, dst) {
        crate::println!("mv: {}", e);
    }
}
