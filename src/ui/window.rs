use super::actions::{self, app_command};
use super::gdi::{wide, Bitmap};
use super::layout::{self, Hit};
use super::{lock, Ctx};
use crate::app::EditMode;
use crate::player::PlayState;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWM_BB_ENABLE,
    DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint,
    InvalidateRect, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HDC, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::HDROP;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyState, ReleaseCapture, SetCapture, SetFocus, VK_CONTROL, VK_DELETE, VK_DOWN,
    VK_END, VK_ESCAPE, VK_F11, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT,
    VK_SPACE, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextW, PostQuitMessage, SendMessageW,
    SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage, ES_AUTOHSCROLL, GWLP_USERDATA, GWLP_WNDPROC,
    GWL_EXSTYLE, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT, HTTOP,
    HTTOPLEFT, HTTOPRIGHT, LWA_ALPHA, MSG, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_DPICHANGED,
    WM_DROPFILES,
    WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCALCSIZE, WM_NCHITTEST, WM_PAINT,
    WM_RBUTTONUP, WM_SIZE, WM_TIMER, WS_CHILD, WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_EX_LAYERED,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,     WS_THICKFRAME, WS_VISIBLE,
};

pub fn run_window(ctx: &Ctx) -> i32 {
    unsafe {
        let instance = GetModuleHandleW(None)
            .map(|m| windows::Win32::Foundation::HINSTANCE(m.0))
            .unwrap_or(windows::Win32::Foundation::HINSTANCE(std::ptr::null_mut()));
        let (x, y, w, h, mini) = {
            let app = lock(&ctx.app);
            if app.mini {
                (app.mini_pos.0, app.mini_pos.1, app.pos.2.max(420), 132, true)
            } else {
                (app.pos.0, app.pos.1, app.pos.2, app.pos.3, false)
            }
        };
        let _ = mini;
        let hwnd = match CreateWindowExW(
            WS_EX_APPWINDOW,
            w!("AirPlayMain"),
            w!("AirPlay"),
            WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU | WS_CLIPCHILDREN,
            x,
            y,
            w,
            h,
            None,
            None,
            instance,
            Some(ctx as *const Ctx as *const _),
        ) {
            Ok(hwnd) => hwnd,
            Err(_) => return 1,
        };
        ctx.main.set(hwnd);
        let edit = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            w!(""),
            WS_CHILD | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            8,
            8,
            200,
            22,
            hwnd,
            None,
            instance,
            None,
        );
        if let Ok(edit) = edit {
            ctx.edit.set(edit);
            let prev = SetWindowLongPtrW(edit, GWLP_WNDPROC, edit_proc as usize as isize);
            ctx.edit_proc.set(prev);
            SetWindowLongPtrW(edit, GWLP_USERDATA, ctx as *const Ctx as isize);
            let _ = ShowWindow(edit, SW_HIDE);
        }
        windows::Win32::UI::Shell::DragAcceptFiles(hwnd, true);
        apply_chrome(ctx, hwnd);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = windows::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 33, None);
        actions::boot(ctx);
        let start_mode = lock(&ctx.app).startup_minimized;
        if start_mode == 1 {
            let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_SHOWMINIMIZED);
        } else if start_mode == 2 && lock(&ctx.app).systray {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        if lock(&ctx.app).desktop_lrc {
            super::desktop::set_visible(ctx, true);
        }
        if lock(&ctx.app).systray {
            super::tray::add(hwnd, "AirPlay");
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 != 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        msg.wParam.0 as i32
    }
}

pub unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_CREATE {
        let cs = &*(lp.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return LRESULT(0);
    }
    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if p == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let ctx = &*(p as *const Ctx);
    const EDIT_DONE: u32 = WM_APP + 8;
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_DPICHANGED => {
            let dpi = ((wp.0 >> 16) & 0xffff) as u32;
            if dpi >= 96 {
                ctx.dpi.set(dpi);
            }
            if lp.0 != 0 {
                let r = &*(lp.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                );
            }
            LRESULT(0)
        }
        WM_NCCALCSIZE => {
            if wp.0 == 0 {
                DefWindowProcW(hwnd, msg, wp, lp)
            } else {
                LRESULT(0)
            }
        }
        WM_NCHITTEST => LRESULT(hit_test(ctx, hwnd, lp) as isize),
        WM_PAINT => {
            paint(ctx, hwnd);
            LRESULT(0)
        }
        WM_TIMER => {
            on_timer(ctx, hwnd);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = point(lp);
            on_down(ctx, hwnd, x, y);
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            let (x, y) = point(lp);
            let hit = hit_at(ctx, hwnd, x, y);
            if matches!(hit, Hit::Track(_) | Hit::Side(_)) {
                actions::click(ctx, hwnd, hit, true);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if (wp.0 & 1) != 0 {
                let (x, y) = point(lp);
                on_drag(ctx, hwnd, x, y);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = point(lp);
            on_up(ctx, hwnd, x, y);
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt).is_ok() {
                super::menu::popup(hwnd, ctx, pt.x, pt.y);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            on_wheel(ctx, hwnd, wp, lp);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            on_key(ctx, hwnd, wp.0 as u16);
            LRESULT(0)
        }
        WM_HOTKEY => {
            actions::hotkey(ctx, hwnd, wp.0 as i32);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xffff) as usize;
            let note = (wp.0 >> 16) & 0xffff;
            if note == 0x0300 {
                LRESULT(0)
            } else if id >= 2200 {
                actions::command(ctx, hwnd, id);
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
        EDIT_DONE => {
            let vk = wp.0 as u16;
            let text = edit_text(ctx.edit.get());
            actions::edit_submit(ctx, &text, vk == VK_ESCAPE.0);
            hide_edit(ctx);
            LRESULT(0)
        }
        super::tray::WM_TRAY => {
            let ev = lp.0 as u32;
            if ev == WM_LBUTTONDBLCLK || ev == WM_LBUTTONUP {
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
            } else if ev == WM_RBUTTONUP {
                let mut pt = POINT::default();
                if GetCursorPos(&mut pt).is_ok() {
                    super::menu::popup(hwnd, ctx, pt.x, pt.y);
                }
            }
            LRESULT(0)
        }
        WM_DROPFILES => {
            drop_files(ctx, HDROP(wp.0 as *mut _));
            LRESULT(0)
        }
        WM_SIZE => {
            remember_pos(ctx, hwnd);
            place_edit(ctx, hwnd);
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_MOVE => {
            remember_pos(ctx, hwnd);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_CLOSE => {
            let tray = lock(&ctx.app).systray && !ctx.quit.get();
            if tray {
                let _ = ShowWindow(hwnd, SW_HIDE);
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
        WM_DESTROY => {
            lock(&ctx.app).save();
            super::tray::remove(hwnd);
            super::hotkey::apply(hwnd, &std::array::from_fn(|_| String::new()), false);
            PostQuitMessage(0);
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_APPCOMMAND => {
            let cmd = (lp.0 >> 16) & 0xffff;
            app_command(ctx, cmd as usize);
            LRESULT(1)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_GETMINMAXINFO => {
            let info = &mut *(lp.0 as *mut windows::Win32::UI::WindowsAndMessaging::MINMAXINFO);
            info.ptMinTrackSize.x = 420;
            info.ptMinTrackSize.y = 120;
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn edit_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if p == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let ctx = &*(p as *const Ctx);
    if msg == WM_KEYDOWN && (wp.0 as u16 == VK_RETURN.0 || wp.0 as u16 == VK_ESCAPE.0) {
        let _ = SendMessageW(ctx.main.get(), WM_APP + 8, wp, LPARAM(0));
        return LRESULT(0);
    }
    let prev = ctx.edit_proc.get();
    if prev == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let f: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT = std::mem::transmute(prev);
    CallWindowProcW(Some(f), hwnd, msg, wp, lp)
}

fn paint(ctx: &Ctx, hwnd: HWND) {
    unsafe {
        sync_dpi(ctx, hwnd);
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let (w, h) = super::paint::client_size(hwnd);
        // 32-bit DIBs ignore per-monitor color depth. On an HDR primary, CreateCompatibleBitmap
        // blit-back can be blank until the window is dragged to a secondary display.
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let created = CreateDIBSection(
            hdc,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            windows::Win32::Foundation::HANDLE(std::ptr::null_mut()),
            0,
        );
        if let Ok(bmp) = created {
            let mem = CreateCompatibleDC(hdc);
            let old = SelectObject(mem, bmp);
            super::paint::draw(ctx, hwnd, mem);
            if !bits.is_null() {
                let n = (w as usize).saturating_mul(h as usize);
                let px = std::slice::from_raw_parts_mut(bits as *mut u32, n);
                for p in px.iter_mut() {
                    *p |= 0xFF00_0000;
                }
            }
            let _ = BitBlt(hdc, 0, 0, w, h, mem, 0, 0, SRCCOPY);
            let _ = SelectObject(mem, old);
            let _ = DeleteObject(bmp);
            let _ = DeleteDC(mem);
        } else {
            super::paint::draw(ctx, hwnd, hdc);
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

fn sync_dpi(ctx: &Ctx, hwnd: HWND) {
    unsafe {
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
        if dpi >= 96 {
            ctx.dpi.set(dpi);
        }
    }
}

fn on_timer(ctx: &Ctx, hwnd: HWND) {
    if ctx.engine.take_ended() {
        actions::on_ended(ctx);
    }
    let st = ctx.engine.status();
    if st.state == PlayState::Playing {
        lock(&ctx.app).note_pos(st.position);
    }
    if !st.error.is_empty() {
        let mut app = lock(&ctx.app);
        if app.note != st.error {
            app.note = st.error.clone();
        }
    }
    sync_visual(ctx);
    if lock(&ctx.app).show_vis {
        let wave = ctx.engine.wave();
        let mut planner = ctx.fft.borrow_mut();
        let next = crate::player::spectrum(&wave, &mut planner);
        drop(planner);
        let mut bars = ctx.bars.borrow_mut();
        for (b, n) in bars.iter_mut().zip(next) {
            *b = if st.state == PlayState::Playing { *b * 0.55 + n * 0.45 } else { *b * 0.8 };
        }
    }
    place_edit(ctx, hwnd);
    taskbar(ctx, hwnd, &st);
    ctx.tick.set(ctx.tick.get().wrapping_add(1));
    if ctx.tick.get() % 150 == 0 {
        lock(&ctx.app).save();
    }
    if ctx.tick.get() % 30 == 0 && lock(&ctx.app).systray {
        let tip = {
            let app = lock(&ctx.app);
            if app.now_title.is_empty() { app.custom_name.clone() } else { app.now_title.clone() }
        };
        super::tray::set_tip(hwnd, &tip);
    }
    unsafe { let _ = InvalidateRect(hwnd, None, false); }
    let desk = ctx.desk.get();
    if !desk.0.is_null() && lock(&ctx.app).desktop_lrc {
        unsafe { let _ = InvalidateRect(desk, None, false); }
    }
}

fn sync_visual(ctx: &Ctx) {
    let (gen, cover, wall) = {
        let app = lock(&ctx.app);
        (app.visuals_gen, app.cover_bytes.clone(), app.wall_path.clone())
    };
    if gen == ctx.seen.get() {
        return;
    }
    ctx.seen.set(gen);
    *ctx.cover.borrow_mut() = cover.as_deref().and_then(super::gdi::decode_image);
    *ctx.wall.borrow_mut() = wall.as_deref().and_then(super::gdi::decode_path);
}

fn on_down(ctx: &Ctx, hwnd: HWND, x: i32, y: i32) {
    let hit = hit_at(ctx, hwnd, x, y);
    match hit {
        Hit::Progress => {
            ctx.drag.set(1);
            unsafe { let _ = SetCapture(hwnd); }
            let ratio = {
                let app = lock(&ctx.app);
                let (w, h) = super::paint::client_size(hwnd);
                let mut layout = layout::compute(&app, w, h, dpi_scale(ctx));
                layout::apply_zones(&mut layout, &app, w, h);
                layout.progress_at(x)
            };
            actions::seek_ratio(ctx, ratio);
        }
        Hit::Volume => {
            ctx.drag.set(2);
            unsafe { let _ = SetCapture(hwnd); }
            let v = volume_at(ctx, hwnd, x);
            actions::set_volume(ctx, v);
        }
        Hit::EqBand(b) => {
            ctx.drag.set(3);
            ctx.drag_band.set(b as u8);
            unsafe { let _ = SetCapture(hwnd); }
            let gain = eq_at(ctx, hwnd, y);
            actions::set_eq_from_y(ctx, b, gain);
        }
        Hit::Track(_) | Hit::Side(_) => actions::click(ctx, hwnd, hit, false),
        _ => {}
    }
}

fn on_drag(ctx: &Ctx, hwnd: HWND, x: i32, y: i32) {
    match ctx.drag.get() {
        1 => {
            let ratio = {
                let app = lock(&ctx.app);
                let (w, h) = super::paint::client_size(hwnd);
                let mut layout = layout::compute(&app, w, h, dpi_scale(ctx));
                layout::apply_zones(&mut layout, &app, w, h);
                layout.progress_at(x)
            };
            actions::seek_ratio(ctx, ratio);
        }
        2 => actions::set_volume(ctx, volume_at(ctx, hwnd, x)),
        3 => actions::set_eq_from_y(ctx, ctx.drag_band.get() as usize, eq_at(ctx, hwnd, y)),
        _ => {}
    }
}

fn on_up(ctx: &Ctx, hwnd: HWND, x: i32, y: i32) {
    if ctx.drag.get() == 0 {
        let hit = hit_at(ctx, hwnd, x, y);
        if !matches!(hit, Hit::Progress | Hit::Volume | Hit::EqBand(_) | Hit::Track(_) | Hit::Side(_)) {
            actions::click(ctx, hwnd, hit, false);
        }
    }
    ctx.drag.set(0);
}

fn on_wheel(ctx: &Ctx, hwnd: HWND, wp: WPARAM, lp: LPARAM) {
    let delta = ((wp.0 >> 16) & 0xffff) as u16 as i16 as i32 / 120;
    let mut pt = POINT {
        x: (lp.0 & 0xffff) as i16 as i32,
        y: ((lp.0 >> 16) & 0xffff) as i16 as i32,
    };
    unsafe { let _ = windows::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut pt); }
    let hit = hit_at(ctx, hwnd, pt.x, pt.y);
    if matches!(hit, Hit::Track(_) | Hit::None) && hit_in_list(ctx, hwnd, pt.x, pt.y) {
        let mut app = lock(&ctx.app);
        app.scroll = (app.scroll - delta).max(0);
    } else if matches!(hit, Hit::Side(_)) {
        let mut app = lock(&ctx.app);
        app.side_scroll = (app.side_scroll - delta).max(0);
    } else {
        actions::volume_by(ctx, delta * 4);
    }
}

fn on_key(ctx: &Ctx, hwnd: HWND, vk: u16) {
    if !ctx.edit.get().0.is_null() {
        unsafe {
            if GetFocus() == ctx.edit.get() {
                return;
            }
        }
    }
    let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
    let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
    match vk {
        v if v == VK_SPACE.0 => actions::toggle(ctx),
        v if v == VK_RETURN.0 => {
            let i = lock(&ctx.app).selected;
            actions::play_index(ctx, i, false);
        }
        v if v == VK_LEFT.0 => actions::seek_by(ctx, -5.0),
        v if v == VK_RIGHT.0 => actions::seek_by(ctx, 5.0),
        v if v == VK_UP.0 && ctrl => actions::volume_by(ctx, 5),
        v if v == VK_DOWN.0 && ctrl => actions::volume_by(ctx, -5),
        v if v == VK_UP.0 => move_sel(ctx, hwnd, -1),
        v if v == VK_DOWN.0 => move_sel(ctx, hwnd, 1),
        v if v == VK_PRIOR.0 => move_sel(ctx, hwnd, -10),
        v if v == VK_NEXT.0 => move_sel(ctx, hwnd, 10),
        v if v == VK_HOME.0 => {
            lock(&ctx.app).selected = 0;
        }
        v if v == VK_END.0 => {
            let mut app = lock(&ctx.app);
            app.selected = app.tracks().len().saturating_sub(1);
        }
        v if v == VK_DELETE.0 => {
            if shift {
                actions::command(ctx, hwnd, super::menu::ID_RECYCLE);
            } else {
                actions::command(ctx, hwnd, super::menu::ID_DEL_TRACK);
            }
        }
        v if v == VK_ESCAPE.0 => {
            if lock(&ctx.app).fullscreen {
                toggle_full(ctx, hwnd);
            } else {
                hide_edit(ctx);
            }
        }
        v if v == VK_F11.0 => toggle_full(ctx, hwnd),
        v if ctrl && v == b'O' as u16 => actions::command(ctx, hwnd, super::menu::ID_ADD_FILE),
        v if ctrl && v == b'F' as u16 => lock(&ctx.app).edit_mode = crate::app::EditMode::Find,
        v if ctrl && v == b'N' as u16 => actions::command(ctx, hwnd, super::menu::ID_NEW_LIST),
        _ => {}
    }
}

fn move_sel(ctx: &Ctx, hwnd: HWND, delta: i32) {
    let mut app = lock(&ctx.app);
    let n = app.tracks().len() as i32;
    if n == 0 {
        return;
    }
    let cur = app.selected as i32;
    app.selected = (cur + delta).clamp(0, n - 1) as usize;
    let (w, h) = super::paint::client_size(hwnd);
    let layout = layout::compute(&app, w, h, dpi_scale(ctx));
    let page = ((layout.list.bottom - layout.list.top) / layout.item_h.max(1)).max(1);
    if (app.selected as i32) < app.scroll {
        app.scroll = app.selected as i32;
    } else if app.selected as i32 >= app.scroll + page {
        app.scroll = app.selected as i32 - page + 1;
    }
}

fn hit_at(ctx: &Ctx, hwnd: HWND, x: i32, y: i32) -> Hit {
    let app = lock(&ctx.app);
    let (w, h) = super::paint::client_size(hwnd);
    let mut layout = layout::compute(&app, w, h, dpi_scale(ctx));
    layout::apply_zones(&mut layout, &app, w, h);
    layout.hit(&app, x, y)
}

fn hit_in_list(ctx: &Ctx, hwnd: HWND, x: i32, y: i32) -> bool {
    let app = lock(&ctx.app);
    let (w, h) = super::paint::client_size(hwnd);
    let layout = layout::compute(&app, w, h, dpi_scale(ctx));
    x >= layout.list.left && y >= layout.list.top && y < layout.list.bottom
}

fn volume_at(ctx: &Ctx, hwnd: HWND, x: i32) -> i32 {
    let app = lock(&ctx.app);
    let (w, h) = super::paint::client_size(hwnd);
    let mut layout = layout::compute(&app, w, h, dpi_scale(ctx));
    layout::apply_zones(&mut layout, &app, w, h);
    layout.volume_at(x)
}

fn eq_at(ctx: &Ctx, hwnd: HWND, y: i32) -> f32 {
    let app = lock(&ctx.app);
    let (w, h) = super::paint::client_size(hwnd);
    let layout = layout::compute(&app, w, h, dpi_scale(ctx));
    layout.eq_gain_at(0, y)
}

fn dpi_scale(ctx: &Ctx) -> f32 {
    ctx.dpi.get().max(96) as f32 / 96.0
}

fn point(lp: LPARAM) -> (i32, i32) {
    let x = (lp.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lp.0 >> 16) & 0xffff) as u16 as i16 as i32;
    (x, y)
}

fn hit_test(ctx: &Ctx, hwnd: HWND, lp: LPARAM) -> u32 {
    let sx = (lp.0 & 0xffff) as u16 as i16 as i32;
    let sy = ((lp.0 >> 16) & 0xffff) as u16 as i16 as i32;
    let mut wr = RECT::default();
    unsafe {
        if GetWindowRect(hwnd, &mut wr).is_err() {
            return HTCLIENT;
        }
    }
    let x = sx - wr.left;
    let y = sy - wr.top;
    let w = wr.right - wr.left;
    let h = wr.bottom - wr.top;
    let b = 6;
    let (left, right, top, bottom) = (x < b, x >= w - b, y < b, y >= h - b);
    if top && left { return HTTOPLEFT; }
    if top && right { return HTTOPRIGHT; }
    if bottom && left { return HTBOTTOMLEFT; }
    if bottom && right { return HTBOTTOMRIGHT; }
    if left { return HTLEFT; }
    if right { return HTRIGHT; }
    if bottom { return HTBOTTOM; }
    if top { return HTTOP; }
    let hit = hit_at(ctx, hwnd, x, y);
    if matches!(hit, Hit::Title | Hit::None) && y < 34 {
        if matches!(hit, Hit::Close | Hit::Min | Hit::Max) {
            return HTCLIENT;
        }
        return HTCAPTION;
    }
    HTCLIENT
}

pub fn apply_chrome(ctx: &Ctx, hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }
    let (top, alpha, blur) = {
        let app = lock(&ctx.app);
        (app.on_top, app.alpha, app.transparent)
    };
    unsafe {
        let insert = if top {
            windows::Win32::UI::WindowsAndMessaging::HWND_TOPMOST
        } else {
            windows::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST
        };
        let _ = SetWindowPos(hwnd, insert, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        let dark = 1u32;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const _, 4);
        // Full-window acrylic composites per-pixel alpha. GDI writes alpha 0, so the window
        // would be fully invisible on the primary display.
        let bb = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE,
            fEnable: windows::Win32::Foundation::BOOL(0),
            hRgnBlur: windows::Win32::Graphics::Gdi::HRGN::default(),
            fTransitionOnMaximized: windows::Win32::Foundation::BOOL(0),
        };
        let _ = DwmEnableBlurBehindWindow(hwnd, &bb);
        let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let layered = if blur { alpha.min(230).max(80) } else { alpha };
        if layered < 255 {
            ex |= WS_EX_LAYERED.0;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), layered, LWA_ALPHA);
        } else {
            ex &= !WS_EX_LAYERED.0;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
        }
        let _ = windows::Win32::Graphics::Gdi::RedrawWindow(
            hwnd,
            None,
            None,
            windows::Win32::Graphics::Gdi::RDW_INVALIDATE
                | windows::Win32::Graphics::Gdi::RDW_UPDATENOW
                | windows::Win32::Graphics::Gdi::RDW_ALLCHILDREN
                | windows::Win32::Graphics::Gdi::RDW_FRAME,
        );
    }
}

pub fn toggle_full(ctx: &Ctx, hwnd: HWND) {
    let on = {
        let mut app = lock(&ctx.app);
        app.fullscreen = !app.fullscreen;
        if app.fullscreen {
            let mut wr = RECT::default();
            unsafe { let _ = GetWindowRect(hwnd, &mut wr); }
            app.restore = Some((wr.left, wr.top, wr.right - wr.left, wr.bottom - wr.top));
        }
        app.fullscreen
    };
    unsafe {
        if on {
            let mon = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
            let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
                cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(mon, &mut info);
            let r = info.rcMonitor;
            let _ = SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER);
        } else if let Some(r) = lock(&ctx.app).restore.take() {
            let _ = SetWindowPos(hwnd, None, r.0, r.1, r.2, r.3, SWP_NOZORDER);
        }
    }
}

pub fn toggle_max(ctx: &Ctx, hwnd: HWND) {
    let on = {
        let mut app = lock(&ctx.app);
        app.maximized = !app.maximized;
        app.maximized
    };
    unsafe {
        let mon = windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
        let mut info = windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(mon, &mut info);
        if on {
            let r = info.rcWork;
            let _ = SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER);
        } else {
            let pos = lock(&ctx.app).pos;
            let _ = SetWindowPos(hwnd, None, pos.0, pos.1, pos.2, pos.3, SWP_NOZORDER);
        }
    }
}

fn remember_pos(ctx: &Ctx, hwnd: HWND) {
    if lock(&ctx.app).fullscreen || lock(&ctx.app).maximized {
        return;
    }
    let mut wr = RECT::default();
    unsafe {
        if GetWindowRect(hwnd, &mut wr).is_err() {
            return;
        }
    }
    let mut app = lock(&ctx.app);
    let w = wr.right - wr.left;
    let h = wr.bottom - wr.top;
    if app.mini {
        app.mini_pos = (wr.left, wr.top);
    } else if w > 50 && h > 50 {
        app.pos = (wr.left, wr.top, w, h);
    }
}

fn place_edit(ctx: &Ctx, hwnd: HWND) {
    let edit = ctx.edit.get();
    if edit.0.is_null() {
        return;
    }
    let mode = lock(&ctx.app).edit_mode;
    unsafe {
        if mode == EditMode::Hidden {
            let _ = ShowWindow(edit, SW_HIDE);
            return;
        }
        let (w, h) = super::paint::client_size(hwnd);
        let _ = SetWindowPos(edit, None, 12, h - 28, (w - 24).max(80), 22, SWP_NOZORDER);
        let _ = ShowWindow(edit, SW_SHOW);
        let _ = SetFocus(edit);
    }
}

fn hide_edit(ctx: &Ctx) {
    lock(&ctx.app).edit_mode = EditMode::Hidden;
    let edit = ctx.edit.get();
    if !edit.0.is_null() {
        unsafe {
            let _ = ShowWindow(edit, SW_HIDE);
            let _ = SetWindowTextW(edit, w!(""));
        }
    }
}

fn edit_text(hwnd: HWND) -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if n <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

fn drop_files(ctx: &Ctx, hdrop: HDROP) {
    unsafe {
        let count = windows::Win32::UI::Shell::DragQueryFileW(hdrop, 0xFFFFFFFF, None);
        let mut paths = Vec::new();
        for i in 0..count {
            let mut buf = vec![0u16; 1024];
            let n = windows::Win32::UI::Shell::DragQueryFileW(hdrop, i, Some(&mut buf));
            if n > 0 {
                paths.push(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n as usize])));
            }
        }
        windows::Win32::UI::Shell::DragFinish(hdrop);
        let first = lock(&ctx.app).add_paths(&paths);
        if let Some(i) = first {
            actions::play_index(ctx, i, false);
        }
    }
}

fn taskbar(ctx: &Ctx, hwnd: HWND, st: &crate::player::Status) {
    if !lock(&ctx.app).taskbar_progress {
        return;
    }
    if ctx.taskbar.borrow().is_none() {
        unsafe {
            if let Ok(tb) = windows::Win32::System::Com::CoCreateInstance::<_, windows::Win32::UI::Shell::ITaskbarList3>(
                &windows::Win32::UI::Shell::TaskbarList,
                None,
                windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
            ) {
                if tb.HrInit().is_ok() {
                    *ctx.taskbar.borrow_mut() = Some(tb);
                }
            }
        }
    }
    let guard = ctx.taskbar.borrow();
    let Some(tb) = guard.as_ref() else {
        return;
    };
    // ITaskbarList3 methods need &self and are unsafe. Clone the interface by... we only have reference.
    // Call through the reference.
    unsafe {
        let completed = (st.position * 1000.0).max(0.0) as u64;
        let total = (st.duration * 1000.0).max(1.0) as u64;
        let _ = tb.SetProgressValue(hwnd, completed, total);
        let state = match st.state {
            PlayState::Playing => windows::Win32::UI::Shell::TBPF_NORMAL,
            PlayState::Paused => windows::Win32::UI::Shell::TBPF_PAUSED,
            PlayState::Stopped => windows::Win32::UI::Shell::TBPF_NOPROGRESS,
        };
        let _ = tb.SetProgressState(hwnd, state);
    }
}

#[allow(dead_code)]
fn _unused(b: Bitmap, c: COLORREF, d: HDC, s: &str) {
    let _ = (b.w, c.0, d.0, s, wide(s));
}
