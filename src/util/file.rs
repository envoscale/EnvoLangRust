//! util/file.envol: reading and writing whole files.

use std::io::{Read, Write};

/// The whole contents of the file at path, or None if it cannot be opened or read. Read in
/// 64 KiB steps, as envoc does.
pub fn read_file(path: &[u8]) -> Option<Vec<u8>> {
    let p = std::str::from_utf8(path).ok()?;
    let mut f = std::fs::File::open(p).ok()?;
    let mut s: Vec<u8> = Vec::new();
    loop {
        s.reserve(1 << 16);
        let len = s.len();
        let cap = s.capacity();
        s.resize(cap, 0);
        let n = match f.read(&mut s[len..]) {
            Ok(n) => n,
            Err(_) => return None,
        };
        s.truncate(len + n);
        if n == 0 {
            break;
        }
    }
    Some(s)
}

/// Writes data to the file at path, replacing it; marks it executable where that means
/// something. Then reads it back, as envoc does: false unless it holds exactly data.
pub fn write_file(path: &[u8], data: &[u8], executable: bool) -> bool {
    let p = match std::str::from_utf8(path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    {
        let mut f = match std::fs::File::create(p) {
            Ok(f) => f,
            Err(_) => return false,
        };
        if f.write_all(data).is_err() {
            return false;
        }
    }
    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = executable;
    match read_file(path) {
        Some(back) => back == data,
        None => false,
    }
}
