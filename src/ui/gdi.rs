use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, DrawTextW, SetBkMode, SetTextColor, StretchDIBits, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_RIGHT, DT_SINGLELINE, DT_VCENTER, SRCCOPY, TRANSPARENT,
};
use windows::Win32::Graphics::Gdi::{HDC, DRAW_TEXT_FORMAT};

pub fn rgb(c: [u8; 3]) -> COLORREF {
    COLORREF((c[0] as u32) | ((c[1] as u32) << 8) | ((c[2] as u32) << 16))
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn fill(hdc: HDC, rect: RECT, color: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(color);
        if !brush.0.is_null() {
            let _ = windows::Win32::Graphics::Gdi::FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);
        }
    }
}

pub fn text(hdc: HDC, value: &str, rect: RECT, color: COLORREF, format: DRAW_TEXT_FORMAT) {
    if value.is_empty() {
        return;
    }
    let mut buf: Vec<u16> = value.encode_utf16().collect();
    unsafe {
        SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, color);
        let _ = DrawTextW(hdc, &mut buf, &mut rect.clone(), format);
    }
}

pub fn flags(parts: &[DRAW_TEXT_FORMAT]) -> DRAW_TEXT_FORMAT {
    let mut v = DRAW_TEXT_FORMAT(0);
    for p in parts {
        v |= *p;
    }
    v
}

pub fn left() -> DRAW_TEXT_FORMAT {
    flags(&[DT_LEFT, DT_VCENTER, DT_SINGLELINE, DT_END_ELLIPSIS, DT_NOPREFIX])
}

pub fn center() -> DRAW_TEXT_FORMAT {
    flags(&[DT_CENTER, DT_VCENTER, DT_SINGLELINE, DT_END_ELLIPSIS, DT_NOPREFIX])
}

pub fn right() -> DRAW_TEXT_FORMAT {
    flags(&[DT_RIGHT, DT_VCENTER, DT_SINGLELINE, DT_END_ELLIPSIS, DT_NOPREFIX])
}

pub fn pt_in(r: RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

pub fn rect(l: i32, t: i32, r: i32, b: i32) -> RECT {
    RECT {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

pub fn inset(r: RECT, dx: i32, dy: i32) -> RECT {
    rect(r.left + dx, r.top + dy, r.right - dx, r.bottom - dy)
}

#[derive(Clone)]
pub struct Bitmap {
    pub w: i32,
    pub h: i32,
    pub bgra: Vec<u8>,
}

pub fn decode_image(bytes: &[u8]) -> Option<Bitmap> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let w = img.width() as i32;
    let h = img.height() as i32;
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for p in img.pixels() {
        bgra.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    Some(Bitmap { w, h, bgra })
}

pub fn decode_path(path: &std::path::Path) -> Option<Bitmap> {
    let bytes = std::fs::read(path).ok()?;
    decode_image(&bytes)
}

pub fn blit(hdc: HDC, bmp: &Bitmap, dest: RECT) {
    if bmp.bgra.is_empty() || dest.right <= dest.left || dest.bottom <= dest.top {
        return;
    }
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bmp.w,
            biHeight: -bmp.h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        let _ = StretchDIBits(
            hdc,
            dest.left,
            dest.top,
            dest.right - dest.left,
            dest.bottom - dest.top,
            0,
            0,
            bmp.w,
            bmp.h,
            Some(bmp.bgra.as_ptr() as *const _),
            &mut info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

#[allow(dead_code)]
pub fn text_width(hdc: HDC, value: &str) -> i32 {
    let mut buf: Vec<u16> = value.encode_utf16().collect();
    let mut r = rect(0, 0, 0, 0);
    unsafe {
        DrawTextW(
            hdc,
            &mut buf,
            &mut r,
            DT_LEFT | DT_SINGLELINE | DT_CALCRECT | DT_NOPREFIX,
        );
    }
    r.right
}
