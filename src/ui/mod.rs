mod actions;
mod desktop;
mod dialogs;
mod gdi;
mod hotkey;
mod layout;
mod menu;
mod paint;
mod settings;
mod tray;
mod window;

use crate::app::App;
use crate::player::AudioEngine;
use rustfft::FftPlanner;
use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND};
use windows::Win32::Graphics::Gdi::{HBRUSH, HFONT};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::ITaskbarList3;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, LoadCursorW, RegisterClassW, SetForegroundWindow, ShowWindow, CS_DBLCLKS,
    CS_HREDRAW, CS_VREDRAW, HCURSOR, IDC_ARROW, SW_RESTORE, WNDCLASSW,
};
use windows::core::{w, PCWSTR};

pub struct Ctx {
    pub app: Mutex<App>,
    pub engine: AudioEngine,
    pub main: Cell<HWND>,
    pub desk: Cell<HWND>,
    pub settings: Cell<HWND>,
    pub edit: Cell<HWND>,
    pub edit_proc: Cell<isize>,
    pub dpi: Cell<u32>,
    pub fft: RefCell<FftPlanner<f32>>,
    pub bars: RefCell<[f32; 48]>,
    pub cover: RefCell<Option<gdi::Bitmap>>,
    pub wall: RefCell<Option<gdi::Bitmap>>,
    pub font: Cell<HFONT>,
    pub font_px: Cell<i32>,
    pub taskbar: RefCell<Option<ITaskbarList3>>,
    pub seen: Cell<u64>,
    pub drag: Cell<u8>,
    pub drag_band: Cell<u8>,
    pub tick: Cell<u32>,
    pub quit: Cell<bool>,
}

pub fn lock(app: &Mutex<App>) -> std::sync::MutexGuard<'_, App> {
    app.lock().unwrap_or_else(|e| e.into_inner())
}

fn null_hwnd() -> HWND {
    HWND(std::ptr::null_mut())
}

pub fn run() -> i32 {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let name = gdi::wide("Local\\AirPlaySingleInstance");
        if let Ok(handle) = CreateMutexW(None, true, PCWSTR(name.as_ptr())) {
            if GetLastError() == ERROR_ALREADY_EXISTS {
                if let Ok(hwnd) = FindWindowW(w!("AirPlayMain"), PCWSTR::null()) {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(hwnd);
                }
                let _ = handle;
                return 0;
            }
            std::mem::forget(handle);
        }
        let instance = match GetModuleHandleW(None) {
            Ok(m) => windows::Win32::Foundation::HINSTANCE(m.0),
            Err(_) => return 1,
        };
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or(HCURSOR(std::ptr::null_mut()));
        for (name, proc) in [
            (
                w!("AirPlayMain"),
                window::wndproc as unsafe extern "system" fn(_, _, _, _) -> _,
            ),
            (w!("AirPlayDeskLrc"), desktop::wndproc),
            (w!("AirPlaySettings"), settings::wndproc),
        ] {
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
                lpfnWndProc: Some(proc),
                hInstance: instance,
                hCursor: cursor,
                hbrBackground: HBRUSH(std::ptr::null_mut()),
                lpszClassName: name,
                ..Default::default()
            };
            if RegisterClassW(&wc) == 0 {
                return 1;
            }
        }
        let ctx = Box::leak(Box::new(Ctx {
            app: Mutex::new(App::load()),
            engine: AudioEngine::new(),
            main: Cell::new(null_hwnd()),
            desk: Cell::new(null_hwnd()),
            settings: Cell::new(null_hwnd()),
            edit: Cell::new(null_hwnd()),
            edit_proc: Cell::new(0),
            dpi: Cell::new(96),
            fft: RefCell::new(FftPlanner::new()),
            bars: RefCell::new([0.0; 48]),
            cover: RefCell::new(None),
            wall: RefCell::new(None),
            font: Cell::new(HFONT(std::ptr::null_mut())),
            font_px: Cell::new(0),
            taskbar: RefCell::new(None),
            seen: Cell::new(0),
            drag: Cell::new(0),
            drag_band: Cell::new(0),
            tick: Cell::new(0),
            quit: Cell::new(false),
        }));
        window::run_window(ctx)
    }
}
