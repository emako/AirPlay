use super::gdi::{blit, center, fill, left, rect, rgb, text};
use super::layout::{self, Layout};
use super::{lock, Ctx};
use crate::player::PlayState;
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreatePen, DeleteObject, LineTo, MoveToEx, SelectObject, PS_SOLID, HDC,
};
use windows::Win32::Graphics::Gdi::CreateFontW;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows::Win32::Foundation::HWND;

pub fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rc = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    ((rc.right - rc.left).max(1), (rc.bottom - rc.top).max(1))
}

pub fn draw(ctx: &Ctx, hwnd: HWND, hdc: HDC) {
    let (w, h) = client_size(hwnd);
    let scale = ctx.dpi.get().max(96) as f32 / 96.0;
    ensure_font(ctx, scale);
    let app = lock(&ctx.app);
    let mut layout = layout::compute(&app, w, h, scale);
    layout::apply_zones(&mut layout, &app, w, h);
    let skin_bg = app.skin().bg;
    let skin_panel = app.skin().panel;
    let skin_text = app.skin().text;
    let skin_dim = app.skin().dim;
    let skin_accent = app.skin().accent;
    let skin_select = app.skin().select;
    let skin_play = app.skin().playing;
    let name = app.custom_name.clone();
    let now = app.now_title.clone();
    let sub = if app.note.is_empty() { app.now_sub.clone() } else { app.note.clone() };
    let st = ctx.engine.status();
    let (line, _) = app.lrc_at(st.position);
    let volume = app.volume;
    let muted = app.muted;
    let random = app.random;
    let repeat_list = app.repeat_list;
    let single = app.track_play;
    let play_label = if st.state == PlayState::Playing { "暂停" } else { "播放" };
    let show_eq = layout.show_eq;
    let gains = app.eq_gains;
    let scroll = app.scroll;
    let side_scroll = app.side_scroll;
    let selected = app.selected;
    let view = app.view;
    let play_list = app.play_list;
    let play_index = app.play_index;
    let list_names: Vec<String> = app.lists.iter().map(|l| l.name.clone()).collect();
    let rows: Vec<(String, bool)> = app
        .tracks()
        .iter()
        .map(|t| (t.title.clone(), t.is_group || t.missing))
        .collect();
    let vis_mode = app.vis_mode;
    drop(app);
    let wall = ctx.wall.borrow();
    if let Some(bmp) = wall.as_ref() {
        blit(hdc, bmp, rect(0, 0, w, h));
    } else {
        fill(hdc, rect(0, 0, w, h), rgb(skin_bg));
    }
    drop(wall);
    unsafe {
        let font = ctx.font.get();
        let old = SelectObject(hdc, font);
        fill(hdc, layout.title, rgb(skin_panel));
        text(hdc, &name, layout.name, rgb(skin_text), left());
        let clock = format!("{} / {}", clock(st.position), clock(st.duration));
        text(hdc, &clock, layout.time, rgb(skin_dim), super::gdi::right());
        btn(hdc, layout.min, "—", false, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.max, "□", false, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.close, "×", false, skin_panel, skin_text, skin_accent);
        if layout.show_cover {
            let cover = ctx.cover.borrow();
            if let Some(bmp) = cover.as_ref() {
                blit(hdc, bmp, layout.cover);
            } else {
                fill(hdc, layout.cover, rgb(skin_select));
            }
        }
        let title = if st.loading {
            "正在打开…"
        } else if !st.error.is_empty() {
            st.error.as_str()
        } else {
            now.as_str()
        };
        text(hdc, title, layout.now, rgb(skin_text), left());
        let sub_line = if layout.show_lrc && !line.is_empty() { line.as_str() } else { sub.as_str() };
        text(hdc, sub_line, layout.sub, rgb(skin_dim), left());
        fill(hdc, layout.progress, rgb(skin_select));
        let ratio = if st.duration > 0.0 { (st.position / st.duration) as f32 } else { 0.0 };
        let pw = ((layout.progress.right - layout.progress.left) as f32 * ratio.clamp(0.0, 1.0)) as i32;
        fill(
            hdc,
            rect(layout.progress.left, layout.progress.top + 4, layout.progress.left + pw, layout.progress.bottom - 4),
            rgb(skin_accent),
        );
        btn(hdc, layout.prev, "上一", false, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.play, play_label, st.state == PlayState::Playing, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.next, "下一", false, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.stop, "停止", false, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.random, if random { "随机" } else { "顺序" }, random, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.repeat_b, if repeat_list { "循环" } else { "循环关" }, repeat_list, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.single, if single { "单曲" } else { "单曲关" }, single, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.add, "添加", false, skin_panel, skin_text, skin_accent);
        btn(hdc, layout.menu, "菜单", false, skin_panel, skin_text, skin_accent);
        fill(hdc, layout.volume, rgb(skin_select));
        let vw = ((layout.volume.right - layout.volume.left) as f32 * if muted { 0.0 } else { volume as f32 / 100.0 }) as i32;
        fill(hdc, rect(layout.volume.left, layout.volume.top, layout.volume.left + vw, layout.volume.bottom), rgb(skin_accent));
        if layout.show_side {
            fill(hdc, layout.side, rgb(skin_panel));
            let page = ((layout.side.bottom - layout.side.top) / layout.side_h.max(1)).max(1);
            for row in 0..page {
                let i = side_scroll + row;
                if i < 0 || i as usize >= list_names.len() { break; }
                let y = layout.side.top + row * layout.side_h;
                let r = rect(layout.side.left, y, layout.side.right, y + layout.side_h);
                if i as usize == view {
                    fill(hdc, r, rgb(skin_select));
                }
                let mut label = list_names[i as usize].clone();
                if play_list == Some(i as usize) {
                    label.insert_str(0, "▶ ");
                }
                text(hdc, &label, super::gdi::inset(r, 8, 0), rgb(skin_text), left());
            }
        }
        if layout.show_list {
            let page = ((layout.list.bottom - layout.list.top) / layout.item_h.max(1)).max(1);
            for row in 0..page {
                let i = scroll + row;
                if i < 0 || i as usize >= rows.len() { break; }
                let y = layout.list.top + row * layout.item_h;
                let r = rect(layout.list.left, y, layout.list.right, y + layout.item_h);
                if i as usize == selected {
                    fill(hdc, r, rgb(skin_select));
                }
                if play_list == Some(view) && play_index == Some(i as usize) {
                    fill(hdc, rect(r.left, r.top, r.left + 3, r.bottom), rgb(skin_play));
                }
                let (title, dim) = &rows[i as usize];
                let color = if *dim { rgb(skin_dim) } else { rgb(skin_text) };
                text(hdc, &format!("{:02}  {title}", i + 1), super::gdi::inset(r, 10, 0), color, left());
            }
        }
        if layout.show_lrc {
            text(hdc, if line.is_empty() { "歌词" } else { line.as_str() }, layout.lrc, rgb(skin_text), center());
        }
        if show_eq {
            draw_eq(hdc, layout.eq, &gains, skin_select, skin_accent, skin_dim);
        }
        if layout.show_vis {
            draw_vis(ctx, hdc, layout.vis, vis_mode, skin_accent, skin_dim, st.state == PlayState::Playing);
        }
        let _ = SelectObject(hdc, old);
    }
}

fn btn(hdc: HDC, r: RECT, label: &str, on: bool, panel: [u8; 3], text_c: [u8; 3], accent: [u8; 3]) {
    fill(hdc, r, if on { rgb(accent) } else { rgb(panel) });
    text(hdc, label, r, if on { COLORREF(0x00101820) } else { rgb(text_c) }, center());
}

fn draw_eq(hdc: HDC, area: RECT, gains: &[f32; 10], track: [u8; 3], accent: [u8; 3], dim: [u8; 3]) {
    let width = (area.right - area.left).max(10);
    let col = width / 10;
    for i in 0..10 {
        let x = area.left + i as i32 * col;
        let bar = rect(x + 4, area.top, x + col - 4, area.bottom);
        fill(hdc, bar, rgb(track));
        let t = ((gains[i] + 15.0) / 30.0).clamp(0.0, 1.0);
        let y = area.bottom - ((area.bottom - area.top) as f32 * t) as i32;
        fill(hdc, rect(bar.left, y, bar.right, area.bottom), rgb(accent));
        text(hdc, &format!("{}", crate::player::EQ_FREQS[i] as i32), rect(x, area.bottom - 14, x + col, area.bottom), rgb(dim), center());
    }
}

fn draw_vis(ctx: &Ctx, hdc: HDC, area: RECT, mode: i32, accent: [u8; 3], dim: [u8; 3], live: bool) {
    let _ = live;
    let bars = *ctx.bars.borrow();
    if mode != 1 {
        let n = bars.len() as i32;
        let width = (area.right - area.left).max(1);
        let bw = (width / n).max(2);
        for (i, v) in bars.iter().enumerate() {
            let x = area.left + i as i32 * bw;
            let h = ((area.bottom - area.top - 8) as f32 * v.clamp(0.0, 1.0)) as i32;
            fill(hdc, rect(x + 1, area.bottom - 4 - h, x + bw - 2, area.bottom - 4), rgb(accent));
        }
    }
    if mode != 0 {
        let wave = ctx.engine.wave();
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, rgb(if mode == 1 { accent } else { dim }));
            let old = SelectObject(hdc, pen);
            let mid = (area.top + area.bottom) / 2;
            let width = (area.right - area.left).max(1);
            let _ = MoveToEx(hdc, area.left, mid, None);
            for x in 0..width {
                let idx = (x as usize * 1024 / width as usize).min(1023);
                let y = mid + (wave[idx] * (area.bottom - area.top) as f32 * 0.45) as i32;
                let _ = LineTo(hdc, area.left + x, y);
            }
            let _ = SelectObject(hdc, old);
            let _ = DeleteObject(pen);
        }
    }
}

fn clock(sec: f64) -> String {
    let sec = if sec.is_finite() { sec.max(0.0) } else { 0.0 };
    let s = sec.round() as i32;
    let m = s / 60;
    let h = m / 60;
    if h > 0 {
        format!("{h}:{:02}:{:02}", m % 60, s % 60)
    } else {
        format!("{m}:{:02}", s % 60)
    }
}

fn ensure_font(ctx: &Ctx, scale: f32) {
    let px = -((15.0 * scale).round() as i32);
    if ctx.font_px.get() == px && !ctx.font.get().0.is_null() {
        return;
    }
    let name = lock(&ctx.app).font_name.clone();
    let wide = super::gdi::wide(&name);
    unsafe {
        let font = CreateFontW(
            px,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            windows::core::PCWSTR(wide.as_ptr()),
        );
        if !font.0.is_null() {
            let old = ctx.font.get();
            if !old.0.is_null() {
                let _ = DeleteObject(old);
            }
            ctx.font.set(font);
            ctx.font_px.set(px);
        }
    }
}
