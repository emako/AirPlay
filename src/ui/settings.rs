use super::gdi::{center, fill, left, rect, rgb, text, wide};
use super::{lock, Ctx};
use crate::player::PRESETS;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, GetStockObject, SelectObject, HDC, DEFAULT_GUI_FONT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, PostQuitMessage, ShowWindow, WS_POPUP,
    WS_VISIBLE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_LBUTTONUP, WM_PAINT,
};
use windows::Win32::Graphics::Gdi::HBRUSH;

const CLASS: &str = "AirPlaySettings";

pub fn toggle(ctx: &Ctx) {
    let hwnd = ctx.settings.get();
    if !hwnd.0.is_null() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
        }
        return;
    }
    let app = lock(&ctx.app);
    let (x, y) = app.settings_pos;
    drop(app);
    unsafe {
        let instance = HINSTANCE();
        if let Ok(hwnd) = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            w!("AirPlaySettings"),
            w!("AirPlay 设置"),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            420,
            510,
            None,
            None,
            instance,
            Some(ctx as *const Ctx as *const _),
        ) {
            ctx.settings.set(hwnd);
        }
    }
}

fn HINSTANCE() -> windows::Win32::Foundation::HINSTANCE {
    unsafe {
        GetModuleHandleW(None)
            .map(|m| windows::Win32::Foundation::HINSTANCE(m.0))
            .unwrap_or(windows::Win32::Foundation::HINSTANCE(std::ptr::null_mut()))
    }
}

pub unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        if let Some(ctx) = ctx_of(hwnd) {
            ctx.settings.set(HWND(std::ptr::null_mut()));
        }
        return LRESULT(0);
    }
    let Some(ctx) = ctx_of(hwnd) else {
        if msg == windows::Win32::UI::WindowsAndMessaging::WM_CREATE {
            let cs = &*(lp.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW);
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                cs.lpCreateParams as isize,
            );
            return LRESULT(0);
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    match msg {
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            draw(ctx, hdc);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let x = (lp.0 & 0xffff) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xffff) as i16 as i32;
            on_click(ctx, hwnd, x, y / 30);
            unsafe {
                windows::Win32::Graphics::Gdi::InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            ctx.settings.set(HWND(std::ptr::null_mut()));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn ctx_of(hwnd: HWND) -> Option<&'static Ctx> {
    let p = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        )
    };
    if p == 0 {
        None
    } else {
        Some(unsafe { &*(p as *const Ctx) })
    }
}

fn draw(ctx: &Ctx, hdc: HDC) {
    let app = lock(&ctx.app);
    let rows = rows(&app);
    drop(app);
    let bg = rgb([28, 30, 36]);
    let panel = rgb([20, 22, 28]);
    fill(hdc, rect(0, 0, 420, 510), bg);
    unsafe {
        let font = GetStockObject(DEFAULT_GUI_FONT);
        let old = SelectObject(hdc, font);
        for (i, (label, value, on)) in rows.iter().enumerate() {
            let y = i as i32 * 30;
            if i % 2 == 0 {
                fill(hdc, rect(0, y, 420, y + 30), panel);
            }
            let color = if *on {
                COLORREF(0x00E8EAED)
            } else {
                COLORREF(0x00888888)
            };
            text(hdc, label, rect(12, y, 250, y + 30), color, left());
            text(hdc, value, rect(250, y, 408, y + 30), COLORREF(0x00C8D0D8), center());
        }
        let _ = SelectObject(hdc, old);
    }
    let _ = wide;
    let _ = PCWSTR::null;
    let _ = PostQuitMessage;
    let _ = CLASS;
}

fn rows(app: &crate::app::App) -> Vec<(String, String, bool)> {
    let dev = if app.device_name.is_empty() {
        "系统默认".into()
    } else {
        app.device_name.clone()
    };
    let preset = PRESETS
        .get(app.eq_preset)
        .map(|p| p.0)
        .unwrap_or("自定义");
    vec![
        ("输出设备（点击切换）".into(), dev, true),
        ("缓冲".into(), format!("{} ms", app.buffer_ms), true),
        (
            "低延迟".into(),
            if app.exclusive { "开（WASAPI 共享）".into() } else { "关".into() },
            true,
        ),
        ("均衡器".into(), format!("{} / {preset}", on(app.eq_on)), true),
        ("包含子目录".into(), on(app.include_subdir).into(), true),
        ("启动自动播放".into(), on(app.auto_play).into(), true),
        ("记住播放进度".into(), on(app.save_position).into(), true),
        ("关闭到托盘".into(), on(app.systray).into(), true),
        ("全局热键".into(), on(app.hk_global).into(), true),
        ("歌词目录开关".into(), on(app.lrc_dir_enabled).into(), true),
        (
            "选择歌词目录".into(),
            if app.lrc_dir.is_empty() { "默认 plugins\\lyrics".into() } else { app.lrc_dir.clone() },
            true,
        ),
        ("模糊".into(), on(app.transparent).into(), true),
        ("窗口置顶".into(), on(app.on_top).into(), true),
        ("任务栏进度".into(), on(app.taskbar_progress).into(), true),
        ("用标签替换列表名".into(), on(app.mod_filename).into(), true),
        ("桌面歌词".into(), on(app.desktop_lrc).into(), true),
        ("关闭".into(), String::new(), true),
    ]
}

fn on(v: bool) -> &'static str {
    if v { "开" } else { "关" }
}

fn on_click(ctx: &Ctx, hwnd: HWND, x: i32, row: i32) {
    let mut app = lock(&ctx.app);
    match row {
        0 => {
            app.devices = crate::player::AudioEngine::devices();
            app.cycle_device();
            let name = app.device_name.clone();
            drop(app);
            ctx.engine.set_device(&name);
        }
        1 => {
            app.cycle_buffer();
            let (ms, low) = (app.buffer_ms, app.exclusive);
            drop(app);
            ctx.engine.set_buffer(ms, low);
        }
        2 => {
            app.exclusive = !app.exclusive;
            let (ms, low) = (app.buffer_ms, app.exclusive);
            drop(app);
            ctx.engine.set_buffer(ms, low);
        }
        3 => {
            if x >= 250 {
                app.cycle_preset();
            } else {
                app.eq_on = !app.eq_on;
            }
            let (on, g) = (app.eq_on, app.eq_gains);
            drop(app);
            ctx.engine.set_eq(on, g);
        }
        4 => app.include_subdir = !app.include_subdir,
        5 => app.auto_play = !app.auto_play,
        6 => app.save_position = !app.save_position,
        7 => app.systray = !app.systray,
        8 => {
            app.hk_global = !app.hk_global;
            let keys = app.hotkeys.clone();
            let on = app.hk_global;
            drop(app);
            super::hotkey::apply(ctx.main.get(), &keys, on);
        }
        9 => app.lrc_dir_enabled = !app.lrc_dir_enabled,
        10 => {
            drop(app);
            if let Some(dir) = super::dialogs::browse_folder(hwnd) {
                lock(&ctx.app).lrc_dir = dir.to_string_lossy().to_string();
            }
        }
        11 => {
            app.transparent = !app.transparent;
            drop(app);
            super::window::apply_chrome(ctx, ctx.main.get());
        }
        12 => {
            app.on_top = !app.on_top;
            drop(app);
            super::window::apply_chrome(ctx, ctx.main.get());
        }
        13 => app.taskbar_progress = !app.taskbar_progress,
        14 => app.mod_filename = !app.mod_filename,
        15 => {
            app.desktop_lrc = !app.desktop_lrc;
            let on = app.desktop_lrc;
            drop(app);
            super::desktop::set_visible(ctx, on);
        }
        16 => {
            if let Ok(mut rc) = window_rect(hwnd) {
                app.settings_pos = (rc.left, rc.top);
            }
            drop(app);
            unsafe { let _ = DestroyWindow(hwnd); }
        }
        _ => {}
    }
}

fn window_rect(hwnd: HWND) -> Result<RECT, ()> {
    let mut rc = RECT::default();
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rc).map_err(|_| ())?;
    }
    Ok(rc)
}
