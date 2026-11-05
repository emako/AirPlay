use encoding_rs::GBK;
use std::path::Path;

pub fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    decode_bytes(&bytes)
}

pub fn decode_bytes(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Some(s.to_string());
    }
    let (cow, _, had_errors) = GBK.decode(bytes);
    if had_errors && cow.trim().is_empty() {
        None
    } else {
        Some(cow.into_owned())
    }
}

pub fn exe_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub fn stem_title(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    match name.rfind('.') {
        Some(i) if i > 0 && !name[i..].contains(['/', '\\']) => name[..i].to_string(),
        _ => name,
    }
}

pub fn is_url(s: &str) -> bool {
    let l = s.trim().to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://")
}
