const MAX_NODES: usize = 128;
const MAX_NAME: usize = 28;
const MAX_CHILDREN: usize = 16;
pub const MAX_FILE_SIZE: usize = 4096;

#[derive(Clone, Copy, PartialEq)]
enum NodeKind {
    Free,
    File,
    Dir,
}

#[derive(Clone, Copy)]
struct Node {
    kind: NodeKind,
    name: [u8; MAX_NAME],
    name_len: usize,
    parent: usize,
    data: [u8; MAX_FILE_SIZE],
    data_len: usize,
    children: [usize; MAX_CHILDREN],
    num_children: usize,
}

const fn empty_node() -> Node {
    Node {
        kind: NodeKind::Free,
        name: [0; MAX_NAME],
        name_len: 0,
        parent: 0,
        data: [0; MAX_FILE_SIZE],
        data_len: 0,
        children: [0; MAX_CHILDREN],
        num_children: 0,
    }
}

static mut NODES: [Node; MAX_NODES] = [empty_node(); MAX_NODES];
static mut CWD: usize = 0;

#[derive(Debug)]
pub enum FsError {
    NotFound,
    NotADir,
    NotAFile,
    AlreadyExists,
    NoSpace,
    NameTooLong,
    DirNotEmpty,
    InvalidPath,
}

impl core::fmt::Display for FsError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            FsError::NotFound => write!(f, "not found"),
            FsError::NotADir => write!(f, "not a directory"),
            FsError::NotAFile => write!(f, "not a file"),
            FsError::AlreadyExists => write!(f, "already exists"),
            FsError::NoSpace => write!(f, "no space left"),
            FsError::NameTooLong => write!(f, "name too long"),
            FsError::DirNotEmpty => write!(f, "directory not empty"),
            FsError::InvalidPath => write!(f, "invalid path"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; MAX_NAME],
    pub name_len: usize,
    pub is_dir: bool,
    pub size: usize,
}

fn alloc_node() -> Result<usize, FsError> {
    unsafe {
        for i in 1..MAX_NODES {
            if NODES[i].kind == NodeKind::Free {
                return Ok(i);
            }
        }
        Err(FsError::NoSpace)
    }
}

fn name_eq(node: usize, name: &str) -> bool {
    unsafe {
        let n = &NODES[node];
        n.name_len == name.len() && &n.name[..n.name_len] == name.as_bytes()
    }
}

fn find_child(dir: usize, name: &str) -> Result<usize, FsError> {
    unsafe {
        let d = &NODES[dir];
        for i in 0..d.num_children {
            if name_eq(d.children[i], name) {
                return Ok(d.children[i]);
            }
        }
        Err(FsError::NotFound)
    }
}

fn resolve(path: &str) -> Result<usize, FsError> {
    if path.is_empty() {
        return Ok(unsafe { CWD });
    }
    let mut cur = if path.starts_with('/') { 0 } else { unsafe { CWD } };
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            cur = unsafe { NODES[cur].parent };
            continue;
        }
        unsafe {
            if NODES[cur].kind != NodeKind::Dir {
                return Err(FsError::NotADir);
            }
        }
        cur = find_child(cur, part)?;
    }
    Ok(cur)
}

fn split_parent(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(0) => ("/", &path[1..]),
        Some(i) => (&path[..i], &path[i + 1..]),
        None => ("", path),
    }
}

fn set_name(idx: usize, name: &str) -> Result<(), FsError> {
    if name.len() > MAX_NAME {
        return Err(FsError::NameTooLong);
    }
    unsafe {
        NODES[idx].name = [0; MAX_NAME];
        NODES[idx].name[..name.len()].copy_from_slice(name.as_bytes());
        NODES[idx].name_len = name.len();
    }
    Ok(())
}

fn add_child(parent: usize, child: usize) -> Result<(), FsError> {
    unsafe {
        let p = &mut NODES[parent];
        if p.num_children >= MAX_CHILDREN {
            return Err(FsError::NoSpace);
        }
        p.children[p.num_children] = child;
        p.num_children += 1;
    }
    Ok(())
}

fn remove_child(parent: usize, child: usize) {
    unsafe {
        let p = &mut NODES[parent];
        for i in 0..p.num_children {
            if p.children[i] == child {
                p.children[i] = p.children[p.num_children - 1];
                p.num_children -= 1;
                return;
            }
        }
    }
}

pub fn init() {
    unsafe {
        NODES[0].kind = NodeKind::Dir;
        NODES[0].name[0] = b'/';
        NODES[0].name_len = 1;
        NODES[0].parent = 0;
    }
    let _ = create_dir("/home");
    let _ = create_dir("/tmp");
    let _ = create_file("/readme.txt");
    let _ = write_file("/readme.txt", b"Welcome to RxV6.\nType 'help' for commands.\n");
}

pub fn create_file(path: &str) -> Result<(), FsError> {
    let (ppath, name) = split_parent(path);
    if name.is_empty() {
        return Err(FsError::InvalidPath);
    }
    let parent = if ppath.is_empty() { unsafe { CWD } } else { resolve(ppath)? };
    unsafe {
        if NODES[parent].kind != NodeKind::Dir {
            return Err(FsError::NotADir);
        }
    }
    if find_child(parent, name).is_ok() {
        return Err(FsError::AlreadyExists);
    }
    let idx = alloc_node()?;
    unsafe {
        NODES[idx].kind = NodeKind::File;
        NODES[idx].parent = parent;
        NODES[idx].data_len = 0;
    }
    set_name(idx, name)?;
    add_child(parent, idx)
}

pub fn create_dir(path: &str) -> Result<(), FsError> {
    let (ppath, name) = split_parent(path);
    if name.is_empty() {
        return Err(FsError::InvalidPath);
    }
    let parent = if ppath.is_empty() { unsafe { CWD } } else { resolve(ppath)? };
    unsafe {
        if NODES[parent].kind != NodeKind::Dir {
            return Err(FsError::NotADir);
        }
    }
    if find_child(parent, name).is_ok() {
        return Err(FsError::AlreadyExists);
    }
    let idx = alloc_node()?;
    unsafe {
        NODES[idx].kind = NodeKind::Dir;
        NODES[idx].parent = parent;
        NODES[idx].num_children = 0;
    }
    set_name(idx, name)?;
    add_child(parent, idx)
}

pub fn read_file(path: &str, buf: &mut [u8]) -> Result<usize, FsError> {
    let idx = resolve(path)?;
    unsafe {
        if NODES[idx].kind != NodeKind::File {
            return Err(FsError::NotAFile);
        }
        let len = NODES[idx].data_len.min(buf.len());
        buf[..len].copy_from_slice(&NODES[idx].data[..len]);
        Ok(len)
    }
}

pub fn write_file(path: &str, data: &[u8]) -> Result<(), FsError> {
    let idx = resolve(path)?;
    unsafe {
        if NODES[idx].kind != NodeKind::File {
            return Err(FsError::NotAFile);
        }
        let len = data.len().min(MAX_FILE_SIZE);
        NODES[idx].data[..len].copy_from_slice(&data[..len]);
        NODES[idx].data_len = len;
    }
    Ok(())
}

pub fn delete(path: &str) -> Result<(), FsError> {
    let idx = resolve(path)?;
    if idx == 0 {
        return Err(FsError::InvalidPath);
    }
    unsafe {
        if NODES[idx].kind == NodeKind::Dir && NODES[idx].num_children > 0 {
            return Err(FsError::DirNotEmpty);
        }
        let parent = NODES[idx].parent;
        remove_child(parent, idx);
        NODES[idx].kind = NodeKind::Free;
    }
    Ok(())
}

pub fn rename(old_path: &str, new_path: &str) -> Result<(), FsError> {
    let idx = resolve(old_path)?;
    if idx == 0 {
        return Err(FsError::InvalidPath);
    }
    let (new_ppath, new_name) = split_parent(new_path);
    let new_parent = if new_ppath.is_empty() { unsafe { CWD } } else { resolve(new_ppath)? };
    unsafe {
        if NODES[new_parent].kind != NodeKind::Dir {
            return Err(FsError::NotADir);
        }
        let old_parent = NODES[idx].parent;
        remove_child(old_parent, idx);
        NODES[idx].parent = new_parent;
    }
    set_name(idx, new_name)?;
    add_child(new_parent, idx)
}

pub fn list_dir(path: &str, entries: &mut [DirEntry]) -> Result<usize, FsError> {
    let idx = resolve(path)?;
    unsafe {
        if NODES[idx].kind != NodeKind::Dir {
            return Err(FsError::NotADir);
        }
        let count = NODES[idx].num_children.min(entries.len());
        for i in 0..count {
            let child = NODES[idx].children[i];
            entries[i].name = NODES[child].name;
            entries[i].name_len = NODES[child].name_len;
            entries[i].is_dir = NODES[child].kind == NodeKind::Dir;
            entries[i].size = NODES[child].data_len;
        }
        Ok(count)
    }
}

pub fn exists(path: &str) -> bool {
    resolve(path).is_ok()
}

pub fn is_directory(path: &str) -> bool {
    match resolve(path) {
        Ok(idx) => unsafe { NODES[idx].kind == NodeKind::Dir },
        Err(_) => false,
    }
}

pub fn set_cwd(path: &str) -> Result<(), FsError> {
    let idx = resolve(path)?;
    unsafe {
        if NODES[idx].kind != NodeKind::Dir {
            return Err(FsError::NotADir);
        }
        CWD = idx;
    }
    Ok(())
}

pub fn cwd_path(buf: &mut [u8]) -> usize {
    unsafe {
        let mut idx = CWD;
        if idx == 0 {
            buf[0] = b'/';
            return 1;
        }
        let mut parts: [usize; 32] = [0; 32];
        let mut depth = 0;
        while idx != 0 && depth < 32 {
            parts[depth] = idx;
            depth += 1;
            idx = NODES[idx].parent;
        }
        let mut pos = 0;
        for i in (0..depth).rev() {
            if pos < buf.len() {
                buf[pos] = b'/';
                pos += 1;
            }
            let n = &NODES[parts[i]];
            let len = n.name_len.min(buf.len() - pos);
            buf[pos..pos + len].copy_from_slice(&n.name[..len]);
            pos += len;
        }
        pos
    }
}
