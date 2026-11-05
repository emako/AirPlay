use super::dialogs;
use super::layout::Hit;
use super::menu;
use super::{lock, Ctx};
use crate::app::EditMode;
use crate::player::PlayState;
use std::path::PathBuf;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    ShowWindow, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
};

pub fn boot(ctx: &Ctx) {
    let (dev, ms, low, vol, eq_on, gains, repeat_one, keys, global, auto, index) = {
        let app = lock(&ctx.app);
        (
            app.device_name.clone(),
            app.buffer_ms,
            app.exclusive,
            app.volume_linear(),
            app.eq_on,
            app.eq_gains,
            app.track_play,
            app.hotkeys.clone(),
            app.hk_global,
            app.auto_play,
            app.selected,
        )
    };
    ctx.engine.set_volume(vol);
    ctx.engine.set_eq(eq_on, gains);
    ctx.engine.set_repeat_one(repeat_one);
    ctx.engine.set_device(&dev);
    ctx.engine.set_buffer(ms, low);
    super::hotkey::apply(ctx.main.get(), &keys, global);
    let args: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| !p.to_string_lossy().starts_with('-'))
        .collect();
    if !args.is_empty() {
        let first = {
            let mut app = lock(&ctx.app);
            app.use_resume = false;
            app.add_paths(&args)
        };
        if let Some(i) = first {
            play_index(ctx, i, false);
        }
        return;
    }
    if auto {
        play_index(ctx, index, true);
    }
}

pub fn command(ctx: &Ctx, hwnd: HWND, id: usize) {
    match id {
        menu::ID_PLAY => toggle(ctx),
        menu::ID_STOP => stop(ctx),
        menu::ID_PREV => prev(ctx),
        menu::ID_NEXT => next(ctx),
        menu::ID_RANDOM => {
            let mut app = lock(&ctx.app);
            app.random = !app.random;
            if app.random {
                app.rebuild_shuffle();
            }
        }
        menu::ID_REPEAT => lock(&ctx.app).repeat_list ^= true,
        menu::ID_SINGLE => {
            let on = {
                let mut app = lock(&ctx.app);
                app.track_play = !app.track_play;
                app.track_play
            };
            ctx.engine.set_repeat_one(on);
        }
        menu::ID_PL_LOOP => lock(&ctx.app).pl_loop ^= true,
        menu::ID_ADD_FILE => add_files(ctx, hwnd),
        menu::ID_ADD_DIR => add_dir(ctx, hwnd),
        menu::ID_ADD_URL => {
            lock(&ctx.app).edit_mode = EditMode::Url;
        }
        menu::ID_NEW_LIST => lock(&ctx.app).new_list(),
        menu::ID_DEL_LIST => {
            stop(ctx);
            lock(&ctx.app).delete_view_list();
        }
        menu::ID_DEL_TRACK => {
            lock(&ctx.app).remove_selected();
        }
        menu::ID_RECYCLE => recycle_selected(ctx, hwnd),
        menu::ID_CLEAR => {
            stop(ctx);
            lock(&ctx.app).clear_view();
        }
        menu::ID_RELOAD => reload(ctx),
        menu::ID_LOCATE => locate(ctx, hwnd),
        menu::ID_FIND => lock(&ctx.app).edit_mode = EditMode::Find,
        menu::ID_SORT => lock(&ctx.app).toggle_sort(),
        menu::ID_GROUP => lock(&ctx.app).toggle_group(),
        menu::ID_LRC => lock(&ctx.app).show_lrc ^= true,
        menu::ID_DESK => {
            let on = {
                let mut app = lock(&ctx.app);
                app.desktop_lrc = !app.desktop_lrc;
                app.desktop_lrc
            };
            super::desktop::set_visible(ctx, on);
        }
        menu::ID_DESK_LOCK => {
            let on = {
                let mut app = lock(&ctx.app);
                app.desk_lock = !app.desk_lock;
                app.desktop_lrc = true;
                app.desk_lock
            };
            let _ = on;
            super::desktop::set_visible(ctx, true);
            if let hwnd = ctx.desk.get() {
                if !hwnd.0.is_null() {
                    super::desktop::set_visible(ctx, true);
                }
            }
        }
        menu::ID_COVER => {
            let mut app = lock(&ctx.app);
            app.show_cover = !app.show_cover;
            app.visuals_gen = app.visuals_gen.wrapping_add(1);
        }
        menu::ID_VIS => {
            let mut app = lock(&ctx.app);
            if app.show_vis {
                app.vis_mode = (app.vis_mode + 1) % 3;
                if app.vis_mode == 0 {
                    app.show_vis = false;
                }
            } else {
                app.show_vis = true;
                app.vis_mode = 0;
            }
        }
        menu::ID_EQ => {
            let mut app = lock(&ctx.app);
            app.show_eq = !app.show_eq;
            if app.show_eq {
                app.eq_on = true;
            }
            let (on, g) = (app.eq_on, app.eq_gains);
            drop(app);
            ctx.engine.set_eq(on, g);
        }
        menu::ID_MINI => toggle_mini(ctx, hwnd),
        menu::ID_TOP => {
            lock(&ctx.app).on_top ^= true;
            super::window::apply_chrome(ctx, hwnd);
        }
        menu::ID_FULL => super::window::toggle_full(ctx, hwnd),
        menu::ID_MAX => super::window::toggle_max(ctx, hwnd),
        menu::ID_SKIN => lock(&ctx.app).cycle_skin(),
        menu::ID_BLUR => {
            lock(&ctx.app).transparent ^= true;
            super::window::apply_chrome(ctx, hwnd);
        }
        menu::ID_SIDE => lock(&ctx.app).show_sidebar ^= true,
        menu::ID_SETTINGS => super::settings::toggle(ctx),
        menu::ID_ABOUT => about(hwnd),
        menu::ID_SHOW => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = ShowWindow(ctx.main.get(), SW_SHOW);
                let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(ctx.main.get());
            }
        }
        menu::ID_EXIT => {
            ctx.quit.set(true);
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(ctx.main.get());
            }
        }
        _ => {}
    }
}

pub fn click(ctx: &Ctx, hwnd: HWND, hit: Hit, dbl: bool) {
    match hit {
        Hit::Play => toggle(ctx),
        Hit::Stop => stop(ctx),
        Hit::Prev => prev(ctx),
        Hit::Next => next(ctx),
        Hit::Random => command(ctx, hwnd, menu::ID_RANDOM),
        Hit::Repeat => command(ctx, hwnd, menu::ID_REPEAT),
        Hit::Single => command(ctx, hwnd, menu::ID_SINGLE),
        Hit::Add => add_files(ctx, hwnd),
        Hit::Menu => {
            let mut pt = windows::Win32::Foundation::POINT::default();
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt).ok(); }
            super::menu::popup(hwnd, ctx, pt.x, pt.y);
        }
        Hit::Close => {
            let tray = lock(&ctx.app).systray;
            unsafe {
                if tray {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                } else {
                    ctx.quit.set(true);
                    let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                }
            }
        }
        Hit::Min => unsafe { let _ = ShowWindow(hwnd, SW_MINIMIZE); },
        Hit::Max => super::window::toggle_max(ctx, hwnd),
        Hit::Track(i) => {
            lock(&ctx.app).selected = i;
            if dbl {
                play_index(ctx, i, false);
            }
        }
        Hit::Side(i) => {
            if dbl {
                lock(&ctx.app).switch_list(i);
                play_index(ctx, 0, false);
            } else {
                lock(&ctx.app).switch_list(i);
            }
        }
        Hit::Cover => command(ctx, hwnd, menu::ID_COVER),
        Hit::EqBand(b) => {
            let y = 0;
            let _ = y;
            let _ = b;
        }
        _ => {}
    }
}

pub fn play_index(ctx: &Ctx, index: usize, resume: bool) {
    let req = {
        let mut app = lock(&ctx.app);
        app.prepare_play(index, resume)
    };
    if let Some(req) = req {
        if req.url {
            ctx.engine.play_url(req.location, req.start, req.end);
        } else {
            ctx.engine.play_file(PathBuf::from(req.location), req.start, req.end);
        }
    }
}

pub fn toggle(ctx: &Ctx) {
    match ctx.engine.status().state {
        PlayState::Playing => ctx.engine.pause(),
        PlayState::Paused => ctx.engine.resume(),
        PlayState::Stopped => {
            let idx = {
                let app = lock(&ctx.app);
                app.play_index.unwrap_or(app.selected)
            };
            play_index(ctx, idx, false);
        }
    }
}

pub fn stop(ctx: &Ctx) {
    ctx.engine.stop();
}

pub fn next(ctx: &Ctx) {
    let idx = lock(&ctx.app).manual_next();
    if let Some(i) = idx {
        play_index(ctx, i, false);
    }
}

pub fn prev(ctx: &Ctx) {
    let st = ctx.engine.status();
    if st.position > 3.0 && st.state != PlayState::Stopped {
        ctx.engine.seek(0.0);
        lock(&ctx.app).note_pos(0.0);
        return;
    }
    let idx = lock(&ctx.app).manual_prev();
    if let Some(i) = idx {
        play_index(ctx, i, false);
    }
}

pub fn on_ended(ctx: &Ctx) {
    let idx = lock(&ctx.app).auto_next();
    if let Some(i) = idx {
        play_index(ctx, i, false);
    }
}

pub fn seek_ratio(ctx: &Ctx, ratio: f32) {
    let dur = ctx.engine.status().duration;
    let t = dur * ratio as f64;
    ctx.engine.seek(t);
    lock(&ctx.app).note_pos(t);
}

pub fn seek_by(ctx: &Ctx, delta: f64) {
    let st = ctx.engine.status();
    let t = (st.position + delta).clamp(0.0, st.duration.max(0.0));
    ctx.engine.seek(t);
    lock(&ctx.app).note_pos(t);
}

pub fn set_volume(ctx: &Ctx, volume: i32) {
    let mut app = lock(&ctx.app);
    app.volume = volume.clamp(0, 100);
    app.muted = false;
    let v = app.volume_linear();
    drop(app);
    ctx.engine.set_volume(v);
}

pub fn volume_by(ctx: &Ctx, delta: i32) {
    let v = lock(&ctx.app).volume + delta;
    set_volume(ctx, v);
}

pub fn toggle_mute(ctx: &Ctx) {
    let v = {
        let mut app = lock(&ctx.app);
        app.muted = !app.muted;
        app.volume_linear()
    };
    ctx.engine.set_volume(v);
}

pub fn hotkey(ctx: &Ctx, hwnd: HWND, id: i32) {
    match id {
        1 | 13 => toggle(ctx),
        2 | 16 => stop(ctx),
        3 | 15 => prev(ctx),
        4 | 14 => next(ctx),
        5 => volume_by(ctx, 5),
        6 => volume_by(ctx, -5),
        7 => toggle_mute(ctx),
        8 => toggle_mini(ctx, hwnd),
        9 => unsafe { let _ = ShowWindow(hwnd, SW_HIDE); },
        10 => command(ctx, hwnd, menu::ID_EXIT),
        11 => command(ctx, hwnd, menu::ID_RANDOM),
        12 => command(ctx, hwnd, menu::ID_SINGLE),
        _ => {}
    }
}

pub fn edit_submit(ctx: &Ctx, text: &str, cancel: bool) {
    let mode = lock(&ctx.app).edit_mode;
    if cancel {
        lock(&ctx.app).edit_mode = EditMode::Hidden;
        return;
    }
    match mode {
        EditMode::Find => {
            let mut app = lock(&ctx.app);
            let q = text.to_string();
            let _ = app.find_next(&q);
            app.edit_mode = EditMode::Hidden;
        }
        EditMode::Url => {
            let url = text.trim().to_string();
            if !url.is_empty() {
                let mut app = lock(&ctx.app);
                let idx = app.add_paths(&[PathBuf::from(&url)]);
                app.edit_mode = EditMode::Hidden;
                drop(app);
                if let Some(i) = idx {
                    play_index(ctx, i, false);
                }
            } else {
                lock(&ctx.app).edit_mode = EditMode::Hidden;
            }
        }
        EditMode::Hidden => {}
    }
}

pub fn set_eq_from_y(ctx: &Ctx, band: usize, gain: f32) {
    let (on, gains) = {
        let mut app = lock(&ctx.app);
        app.set_eq_band(band, gain);
        (app.eq_on, app.eq_gains)
    };
    ctx.engine.set_eq(on, gains);
}

fn add_files(ctx: &Ctx, hwnd: HWND) {
    let dir = lock(&ctx.app).open_dir.clone();
    if let Some(files) = dialogs::open_files(hwnd, &dir) {
        if let Some(parent) = files.first().and_then(|p| p.parent()) {
            lock(&ctx.app).open_dir = parent.to_string_lossy().to_string();
        }
        let first = lock(&ctx.app).add_paths(&files);
        if let Some(i) = first {
            lock(&ctx.app).selected = i;
        }
    }
}

fn add_dir(ctx: &Ctx, hwnd: HWND) {
    if let Some(dir) = dialogs::browse_folder(hwnd) {
        let first = {
            let mut app = lock(&ctx.app);
            let n = app.tracks().len();
            app.add_folder(&dir);
            if app.tracks().len() > n { Some(n) } else { None }
        };
        if let Some(i) = first {
            lock(&ctx.app).selected = i;
        }
    }
}

fn recycle_selected(ctx: &Ctx, hwnd: HWND) {
    let path = lock(&ctx.app).locate_selected();
    if let Some(path) = path {
        if dialogs::recycle(hwnd, &path) {
            lock(&ctx.app).remove_selected();
        }
    }
}

fn locate(ctx: &Ctx, hwnd: HWND) {
    let Some(path) = lock(&ctx.app).locate_selected() else { return };
    let param = super::gdi::wide(&format!("/select,\"{}\"", path.display()));
    unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            hwnd,
            windows::core::w!("open"),
            windows::core::w!("explorer.exe"),
            windows::core::PCWSTR(param.as_ptr()),
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
}

fn reload(ctx: &Ctx) {
    let mut app = lock(&ctx.app);
    let path = app.lists.get(app.view).map(|l| l.path.clone());
    let exe = app.exe_dir.clone();
    if let Some(path) = path {
        let fresh = crate::playlist::Playlist::load(&path, &exe);
        let view = app.view;
        if let Some(slot) = app.lists.get_mut(view) {
            *slot = fresh;
        }
    }
}

fn toggle_mini(ctx: &Ctx, hwnd: HWND) {
    let (mini, pos, mini_pos) = {
        let mut app = lock(&ctx.app);
        app.mini = !app.mini;
        (app.mini, app.pos, app.mini_pos)
    };
    unsafe {
        if mini {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                None,
                mini_pos.0,
                mini_pos.1,
                pos.2.max(420),
                132,
                SWP_NOZORDER(),
            );
        } else {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                None,
                pos.0,
                pos.1,
                pos.2,
                pos.3,
                SWP_NOZORDER(),
            );
        }
    }
}

fn about(hwnd: HWND) {
    let text = super::gdi::wide(
        "AirPlay\n\nWindows 音乐播放器，单个 exe。\n解码使用 symphonia，输出使用 WASAPI 共享模式。",
    );
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            hwnd,
            windows::core::PCWSTR(text.as_ptr()),
            windows::core::w!("关于 AirPlay"),
            windows::Win32::UI::WindowsAndMessaging::MB_OK,
        );
    }
}

fn SWP_NOZORDER() -> windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS {
    windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
}

pub fn app_command(ctx: &Ctx, cmd: usize) {
    match cmd {
        11 => next(ctx),
        12 => prev(ctx),
        13 => stop(ctx),
        14 => toggle(ctx),
        _ => {}
    }
}
