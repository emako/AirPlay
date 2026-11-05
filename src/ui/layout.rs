use super::gdi::{pt_in, rect};
use crate::app::App;
use windows::Win32::Foundation::RECT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    None,
    Title,
    Close,
    Min,
    Max,
    Progress,
    Prev,
    Play,
    Next,
    Stop,
    Random,
    Repeat,
    Single,
    Add,
    Volume,
    Menu,
    Skin,
    Mini,
    Top,
    Lrc,
    Vis,
    Eq,
    Cover,
    Side(usize),
    Track(usize),
    EqBand(usize),
}

#[derive(Clone, Copy)]
pub struct Layout {
    pub title: RECT,
    pub close: RECT,
    pub min: RECT,
    pub max: RECT,
    pub time: RECT,
    pub name: RECT,
    pub cover: RECT,
    pub now: RECT,
    pub sub: RECT,
    pub progress: RECT,
    pub prev: RECT,
    pub play: RECT,
    pub next: RECT,
    pub stop: RECT,
    pub random: RECT,
    pub repeat_b: RECT,
    pub single: RECT,
    pub add: RECT,
    pub volume: RECT,
    pub menu: RECT,
    pub side: RECT,
    pub list: RECT,
    pub lrc: RECT,
    pub vis: RECT,
    pub eq: RECT,
    pub item_h: i32,
    pub side_h: i32,
    pub show_side: bool,
    pub show_list: bool,
    pub show_lrc: bool,
    pub show_vis: bool,
    pub show_eq: bool,
    pub show_cover: bool,
}

pub fn compute(app: &App, width: i32, height: i32, scale: f32) -> Layout {
    let s = |v: i32| ((v as f32) * scale).round() as i32;
    let title_h = s(34);
    let info_h = if app.mini { 0 } else { s(52) };
    let prog_h = s(14);
    let ctrl_h = s(36);
    let lrc_h = if app.show_lrc && !app.mini { s(28) } else { 0 };
    let vis_h = if app.show_vis && !app.mini { s(78) } else { 0 };
    let eq_h = if app.show_eq && !app.mini { s(74) } else { 0 };
    let mut bottom = height;
    let vis = if vis_h > 0 {
        bottom -= vis_h;
        rect(0, bottom, width, bottom + vis_h)
    } else {
        rect(0, 0, 0, 0)
    };
    let eq = if eq_h > 0 {
        bottom -= eq_h;
        rect(s(8), bottom + s(4), width - s(8), bottom + eq_h - s(4))
    } else {
        rect(0, 0, 0, 0)
    };
    let lrc = if lrc_h > 0 {
        bottom -= lrc_h;
        rect(s(12), bottom, width - s(12), bottom + lrc_h)
    } else {
        rect(0, 0, 0, 0)
    };
    let top = title_h + info_h + prog_h + ctrl_h;
    let list_area = rect(0, top, width, bottom.max(top));
    let show_side = app.show_sidebar && !app.mini && width > s(560);
    let side_w = if show_side { s(148) } else { 0 };
    let side = if show_side {
        rect(0, list_area.top, side_w, list_area.bottom)
    } else {
        rect(0, 0, 0, 0)
    };
    let list = if app.mini {
        rect(0, 0, 0, 0)
    } else {
        rect(side_w, list_area.top, width, list_area.bottom)
    };
    let bw = s(28);
    let close = rect(width - bw - s(6), s(4), width - s(6), s(4) + bw);
    let max = rect(close.left - bw - s(4), close.top, close.left - s(4), close.bottom);
    let min = rect(max.left - bw - s(4), close.top, max.left - s(4), close.bottom);
    let time = rect(min.left - s(92), 0, min.left - s(6), title_h);
    let name = rect(s(12), 0, time.left, title_h);
    let show_cover = app.show_cover && !app.mini;
    let cover = if show_cover {
        rect(s(10), title_h + s(6), s(10) + info_h - s(10), title_h + info_h - s(4))
    } else {
        rect(0, 0, 0, 0)
    };
    let text_l = if show_cover { cover.right + s(8) } else { s(12) };
    let now = rect(text_l, title_h + s(4), width - s(12), title_h + s(28));
    let sub = rect(text_l, title_h + s(28), width - s(12), title_h + info_h);
    let progress = rect(s(12), title_h + info_h, width - s(12), title_h + info_h + prog_h);
    let row_y = progress.bottom + s(4);
    let row_b = row_y + ctrl_h - s(8);
    let labels_w = width - s(24) - s(120) - s(64);
    let n = 8;
    let bw_btn = (labels_w / n).clamp(s(40), s(68));
    let mut x = s(8);
    let btn = |x: &mut i32| {
        let r = rect(*x, row_y, *x + bw_btn - s(4), row_b);
        *x += bw_btn;
        r
    };
    let prev = btn(&mut x);
    let play = btn(&mut x);
    let next = btn(&mut x);
    let stop = btn(&mut x);
    let random = btn(&mut x);
    let repeat_b = btn(&mut x);
    let single = btn(&mut x);
    let add = btn(&mut x);
    let menu = rect(width - s(64), row_y, width - s(8), row_b);
    let volume = rect(menu.left - s(116), row_y + s(8), menu.left - s(8), row_b - s(6));
    Layout {
        title: rect(0, 0, width, title_h),
        close,
        min,
        max,
        time,
        name,
        cover,
        now,
        sub,
        progress,
        prev,
        play,
        next,
        stop,
        random,
        repeat_b,
        single,
        add,
        volume,
        menu,
        side,
        list,
        lrc,
        vis,
        eq,
        item_h: s(24),
        side_h: s(26),
        show_side,
        show_list: !app.mini,
        show_lrc: lrc_h > 0,
        show_vis: vis_h > 0,
        show_eq: eq_h > 0,
        show_cover,
    }
}

impl Layout {
    pub fn hit(&self, app: &App, x: i32, y: i32) -> Hit {
        if pt_in(self.close, x, y) {
            return Hit::Close;
        }
        if pt_in(self.max, x, y) {
            return Hit::Max;
        }
        if pt_in(self.min, x, y) {
            return Hit::Min;
        }
        if pt_in(self.progress, x, y) {
            return Hit::Progress;
        }
        if pt_in(self.prev, x, y) {
            return Hit::Prev;
        }
        if pt_in(self.play, x, y) {
            return Hit::Play;
        }
        if pt_in(self.next, x, y) {
            return Hit::Next;
        }
        if pt_in(self.stop, x, y) {
            return Hit::Stop;
        }
        if pt_in(self.random, x, y) {
            return Hit::Random;
        }
        if pt_in(self.repeat_b, x, y) {
            return Hit::Repeat;
        }
        if pt_in(self.single, x, y) {
            return Hit::Single;
        }
        if pt_in(self.add, x, y) {
            return Hit::Add;
        }
        if pt_in(self.volume, x, y) {
            return Hit::Volume;
        }
        if pt_in(self.menu, x, y) {
            return Hit::Menu;
        }
        if self.show_cover && pt_in(self.cover, x, y) {
            return Hit::Cover;
        }
        if self.show_eq && pt_in(self.eq, x, y) {
            let w = (self.eq.right - self.eq.left) / 10;
            if w > 0 {
                let i = ((x - self.eq.left) / w).clamp(0, 9) as usize;
                return Hit::EqBand(i);
            }
        }
        if self.show_side && pt_in(self.side, x, y) {
            let i = app.side_scroll + (y - self.side.top) / self.side_h.max(1);
            if i >= 0 && (i as usize) < app.lists.len() {
                return Hit::Side(i as usize);
            }
        }
        if self.show_list && pt_in(self.list, x, y) {
            let i = app.scroll + (y - self.list.top) / self.item_h.max(1);
            if i >= 0 && (i as usize) < app.tracks().len() {
                return Hit::Track(i as usize);
            }
        }
        if pt_in(self.title, x, y) || pt_in(self.now, x, y) {
            return Hit::Title;
        }
        Hit::None
    }

    pub fn eq_gain_at(&self, band: usize, y: i32) -> f32 {
        let t = (self.eq.bottom - y) as f32 / (self.eq.bottom - self.eq.top).max(1) as f32;
        (t.clamp(0.0, 1.0) * 30.0 - 15.0).clamp(-15.0, 15.0)
    }

    pub fn progress_at(&self, x: i32) -> f32 {
        let w = (self.progress.right - self.progress.left).max(1) as f32;
        ((x - self.progress.left) as f32 / w).clamp(0.0, 1.0)
    }

    pub fn volume_at(&self, x: i32) -> i32 {
        let w = (self.volume.right - self.volume.left).max(1) as f32;
        ((x - self.volume.left) as f32 / w * 100.0).round().clamp(0.0, 100.0) as i32
    }
}

pub fn scale_zone(zone: (i32, i32, i32, i32), design: (i32, i32), w: i32, h: i32) -> RECT {
    let sx = w as f32 / design.0.max(1) as f32;
    let sy = h as f32 / design.1.max(1) as f32;
    rect(
        (zone.0 as f32 * sx) as i32,
        (zone.1 as f32 * sy) as i32,
        ((zone.0 + zone.2) as f32 * sx) as i32,
        ((zone.1 + zone.3) as f32 * sy) as i32,
    )
}

pub fn apply_zones(layout: &mut Layout, app: &App, w: i32, h: i32) {
    let skin = app.skin();
    if skin.zones.is_empty() {
        return;
    }
    let d = skin.design;
    let map = |name: &str| skin.zones.get(name).copied().map(|z| scale_zone(z, d, w, h));
    if let Some(r) = map("close") {
        layout.close = r;
    }
    if let Some(r) = map("min") {
        layout.min = r;
    }
    if let Some(r) = map("max") {
        layout.max = r;
    }
    if let Some(r) = map("prev") {
        layout.prev = r;
    }
    if let Some(r) = map("play") {
        layout.play = r;
    }
    if let Some(r) = map("next") {
        layout.next = r;
    }
    if let Some(r) = map("progress") {
        layout.progress = r;
    }
    if let Some(r) = map("volume") {
        layout.volume = r;
    }
}
