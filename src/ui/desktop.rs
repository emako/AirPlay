use super::gdi::{center, fill, rect, rgb, text};
use super::{lock, Ctx};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint, GetDC,
    GetStockObject, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    DIB_RGB_COLORS, DEFAULT_GUI_FONT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, GetWindowRect, SendMessageW,
    SetWindowLongPtrW, ShowWindow, UpdateLayeredWindow, CREATESTRUCTW, GWLP_USERDATA, GWL_EXSTYLE,
    HTCAPTION, SW_HIDE, SW_SHOW, ULW_ALPHA, WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_RBUTTONUP, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WINDOW_EX_STYLE,
};

pub fn set_visible(ctx: &Ctx, on: bool) {
    let hwnd = ctx.desk.get();
    if on {
        if hwnd.0.is_null() {
            open(ctx);
        } else {
            unsafe { let _ = ShowWindow(hwnd, SW_SHOW); }
            sync_style(ctx, hwnd);
        }
    } else if !hwnd.0.is_null() {
        unsafe { let _ = ShowWindow(hwnd, SW_HIDE); }
    }
}

pub fn open(ctx: &Ctx) {
    let (x, y) = lock(&ctx.app).desk_pos;
    unsafe {
        let instance = GetModuleHandleW(None)
            .map(|m| windows::Win32::Foundation::HINSTANCE(m.0))
            .unwrap_or(windows::Win32::Foundation::HINSTANCE(std::ptr::null_mut()));
        if let Ok(hwnd) = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            w!("AirPlayDeskLrc"),
            w!(""),
            WS_POPUP,
            x,
            y,
            640,
            84,
            None,
            None,
            instance,
            Some(ctx as *const Ctx as *const _),
        ) {
            ctx.desk.set(hwnd);
            let dark = 1u32;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark as *const u32 as *const _,
                4,
            );
            sync_style(ctx, hwnd);
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }
}

fn sync_style(ctx: &Ctx, hwnd: HWND) {
    let lock_through = lock(&ctx.app).desk_lock;
    unsafe {
        let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        ex |= WS_EX_LAYERED.0 | WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0;
        if lock_through {
            ex |= WS_EX_TRANSPARENT.0;
        } else {
            ex &= !WS_EX_TRANSPARENT.0;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
    }
}

pub unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == windows::Win32::UI::WindowsAndMessaging::WM_CREATE {
        let cs = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(0);
    }
    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if p == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let ctx = &*(p as *const Ctx);
    match msg {
        WM_PAINT => {
            let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            paint_layered(ctx, hwnd);
            LRESULT(0)
        }
        WM_NCHITTEST => LRESULT(HTCAPTION as isize),
        WM_LBUTTONDOWN => {
            let _ = ReleaseCapture();
            let _ = SendMessageW(hwnd, WM_NCHITTEST, WPARAM(0), lp);
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN,
                WPARAM(HTCAPTION as usize),
                lp,
            );
            LRESULT(0)
        }
        WM_LBUTTONUP | WM_MOUSEMOVE => {
            if msg == WM_LBUTTONUP {
                save_pos(ctx, hwnd);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_RBUTTONUP => {
            let mut app = lock(&ctx.app);
            app.desk_lock = !app.desk_lock;
            drop(app);
            sync_style(ctx, hwnd);
            LRESULT(0)
        }
        WM_CLOSE | WM_DESTROY => {
            if msg == WM_CLOSE {
                lock(&ctx.app).desktop_lrc = false;
                let _ = ShowWindow(hwnd, SW_HIDE);
                LRESULT(0)
            } else {
                ctx.desk.set(HWND(std::ptr::null_mut()));
                LRESULT(0)
            }
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn paint_layered(ctx: &Ctx, hwnd: HWND) {
    let (title, next) = {
        let app = lock(&ctx.app);
        let pos = ctx.engine.status().position;
        let (cur, nxt) = app.lrc_at(pos);
        let title = if cur.is_empty() { app.now_title.clone() } else { cur };
        (title, nxt)
    };
    const W: i32 = 640;
    const H: i32 = 84;
    unsafe {
        let screen = GetDC(HWND(std::ptr::null_mut()));
        let mem = CreateCompatibleDC(screen);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: W,
                biHeight: -H,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(
            screen,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            windows::Win32::Foundation::HANDLE(std::ptr::null_mut()),
            0,
        ) else {
            let _ = DeleteDC(mem);
            ReleaseDC(HWND(std::ptr::null_mut()), screen);
            return;
        };
        let old = SelectObject(mem, bmp);
        fill(mem, rect(0, 0, W, H), COLORREF(0));
        let font = GetStockObject(DEFAULT_GUI_FONT);
        let old_font = SelectObject(mem, font);
        text(mem, &title, rect(8, 6, W - 8, 48), COLORREF(0x00FFFFFF), center());
        text(mem, &next, rect(8, 46, W - 8, H - 6), COLORREF(0x00DDDDDD), center());
        let _ = SelectObject(mem, old_font);
        if !bits.is_null() {
            let px = std::slice::from_raw_parts_mut(bits as *mut u32, (W * H) as usize);
            for p in px.iter_mut() {
                let b = *p & 0xff;
                let g = (*p >> 8) & 0xff;
                let r = (*p >> 16) & 0xff;
                let a = r.max(g).max(b);
                *p = b | (g << 8) | (r << 16) | (a << 24);
            }
        }
        let size = SIZE { cx: W, cy: H };
        let origin = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            screen,
            None,
            Some(&size),
            mem,
            Some(&origin),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
        let _ = SelectObject(mem, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
        ReleaseDC(HWND(std::ptr::null_mut()), screen);
    }
}

fn save_pos(ctx: &Ctx, hwnd: HWND) {
    let mut rc = windows::Win32::Foundation::RECT::default();
    unsafe {
        if GetWindowRect(hwnd, &mut rc).is_ok() {
            lock(&ctx.app).desk_pos = (rc.left, rc.top);
        }
    }
}

#[allow(dead_code)]
fn _keep(p: POINT, s: WINDOW_EX_STYLE) -> bool {
    p.x == 0 && s.0 == 0
}
