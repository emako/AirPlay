use super::gdi::wide;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_ALLOWMULTISELECT, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY,
    OPENFILENAMEW,
};
use windows::Win32::UI::Shell::{
    SHBrowseForFolderW, SHFileOperationW, SHGetPathFromIDListW, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
    FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONQUESTION, MB_YESNO, IDYES};
use std::path::PathBuf;

pub fn open_files(owner: HWND, initial: &str) -> Option<Vec<PathBuf>> {
    let mut buf = vec![0u16; 32768];
    let filter = wide("音频\0*.mp3;*.flac;*.wav;*.ogg;*.m4a;*.aac;*.aiff;*.cue;*.m3u;*.m3u8;*.wma;*.ape;*.opus\0所有文件\0*.*\0");
    let title = wide("添加音频");
    let dir = wide(initial);
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrInitialDir: if initial.is_empty() {
            PCWSTR::null()
        } else {
            PCWSTR(dir.as_ptr())
        },
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_EXPLORER | OFN_ALLOWMULTISELECT | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY,
        ..Default::default()
    };
    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if ok.0 == 0 {
        return None;
    }
    let parts = split_multi(&buf);
    if parts.is_empty() {
        return None;
    }
    if parts.len() == 1 {
        return Some(vec![PathBuf::from(&parts[0])]);
    }
    let dir = PathBuf::from(&parts[0]);
    Some(parts.into_iter().skip(1).map(|n| dir.join(n)).collect())
}

pub fn browse_folder(owner: HWND) -> Option<PathBuf> {
    let title = wide("选择文件夹");
    let mut name = [0u16; 260];
    let info = BROWSEINFOW {
        hwndOwner: owner,
        pidlRoot: std::ptr::null_mut(),
        pszDisplayName: PWSTR(name.as_mut_ptr()),
        lpszTitle: PCWSTR(title.as_ptr()),
        ulFlags: BIF_RETURNONLYFSDIRS,
        lpfn: None,
        lParam: LPARAM(0),
        iImage: 0,
    };
    let pidl = unsafe { SHBrowseForFolderW(&info) };
    if pidl.is_null() {
        return None;
    }
    let mut path = [0u16; 260];
    let ok = unsafe { SHGetPathFromIDListW(pidl, &mut path) };
    unsafe { CoTaskMemFree(Some(pidl as *const _)) };
    if ok.0 == 0 {
        return None;
    }
    let n = path.iter().position(|c| *c == 0).unwrap_or(path.len());
    Some(PathBuf::from(String::from_utf16_lossy(&path[..n])))
}

pub fn recycle(owner: HWND, path: &std::path::Path) -> bool {
    let question = wide(&format!("把这个文件移到回收站？\n{}", path.display()));
    let answer = unsafe {
        MessageBoxW(
            owner,
            PCWSTR(question.as_ptr()),
            PCWSTR(wide("AirPlay").as_ptr()),
            MB_YESNO | MB_ICONQUESTION,
        )
    };
    if answer.0 != IDYES.0 {
        return false;
    }
    let mut from = wide(&path.to_string_lossy());
    from.push(0);
    let mut op = SHFILEOPSTRUCTW {
        hwnd: owner,
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        pTo: PCWSTR::null(),
        fFlags: (FOF_ALLOWUNDO.0 | FOF_NOCONFIRMATION.0 | FOF_SILENT.0 | FOF_NOERRORUI.0) as u16,
        ..Default::default()
    };
    unsafe { SHFileOperationW(&mut op) == 0 }
}

fn split_multi(buf: &[u16]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = Vec::new();
    for &c in buf {
        if c == 0 {
            if cur.is_empty() {
                break;
            }
            parts.push(String::from_utf16_lossy(&cur));
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    parts
}
