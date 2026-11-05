use crate::config::Ini;
use crate::cue::parse_cue;
use crate::lrc::{self, LrcLine};
use crate::meta::{self, display_now, Meta};
use crate::player::PRESETS;
use crate::playlist::{self, is_audio_file, is_list_file, Playlist};
use crate::skin::{self, colorref_rgb, Skin};
use crate::textutil::{exe_dir, is_url, read_text, stem_title};
use crate::track::Track;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMode {
    Hidden,
    Find,
    Url,
}

pub struct PlayReq {
    pub location: String,
    pub start: f64,
    pub end: Option<f64>,
    pub url: bool,
}

pub struct App {
    pub exe_dir: PathBuf,
    pub ini_path: PathBuf,
    pub ini: Ini,
    pub custom_name: String,
    pub lists: Vec<Playlist>,
    pub view: usize,
    pub selected: usize,
    pub scroll: i32,
    pub side_scroll: i32,
    pub play_list: Option<usize>,
    pub play_index: Option<usize>,
    pub shuffle: Vec<usize>,
    pub shuffle_pos: usize,
    pub random: bool,
    pub repeat_list: bool,
    pub track_play: bool,
    pub pl_loop: bool,
    pub volume: i32,
    pub muted: bool,
    pub eq_on: bool,
    pub eq_preset: usize,
    pub eq_gains: [f32; 10],
    pub show_lrc: bool,
    pub show_cover: bool,
    pub show_vis: bool,
    pub vis_mode: i32,
    pub show_eq: bool,
    pub show_sidebar: bool,
    pub mini: bool,
    pub on_top: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub transparent: bool,
    pub systray: bool,
    pub include_subdir: bool,
    pub save_position: bool,
    pub auto_play: bool,
    pub taskbar_progress: bool,
    pub mod_filename: bool,
    pub lrc_dir_enabled: bool,
    pub lrc_dir: String,
    pub exclusive: bool,
    pub buffer_ms: u32,
    pub device_name: String,
    pub devices: Vec<String>,
    pub hk_global: bool,
    pub hotkeys: [String; 12],
    pub font_name: String,
    pub alpha: u8,
    pub pos: (i32, i32, i32, i32),
    pub mini_pos: (i32, i32),
    pub desk_pos: (i32, i32),
    pub settings_pos: (i32, i32),
    pub open_dir: String,
    pub resume_tenths: i32,
    pub use_resume: bool,
    pub now_title: String,
    pub now_sub: String,
    pub lrc: Vec<LrcLine>,
    pub cover_bytes: Option<Vec<u8>>,
    pub wall_path: Option<PathBuf>,
    pub visuals_gen: u64,
    pub skins: Vec<Skin>,
    pub skin_index: usize,
    pub note: String,
    pub edit_mode: EditMode,
    pub desktop_lrc: bool,
    pub desk_lock: bool,
    pub startup_minimized: i32,
    pub rng: u64,
    pub restore: Option<(i32, i32, i32, i32)>,
}

impl App {
    pub fn load() -> Self {
        let exe_dir = exe_dir();
        let ini_path = exe_dir.join("airplay.ini");
        let ini = Ini::load(&ini_path);
        let mut app = Self {
            exe_dir: exe_dir.clone(),
            ini_path,
            custom_name: ini.get("main", "custom_name").unwrap_or_else(|| "AirPlay".into()),
            lists: Vec::new(),
            view: 0,
            selected: 0,
            scroll: 0,
            side_scroll: 0,
            play_list: None,
            play_index: None,
            shuffle: Vec::new(),
            shuffle_pos: 0,
            random: ini.get_bool("main", "random", false),
            repeat_list: ini.get_bool("main", "repeat", false),
            track_play: ini.get_bool("main", "track_play", false),
            pl_loop: ini.get_bool("main", "pl_loop", false),
            volume: ini.get_i32("main", "volume_tune", 80).clamp(0, 100),
            muted: false,
            eq_on: ini.get_bool("main", "eq_on", false),
            eq_preset: ini.get_i32("main", "eq_preset", 0).max(0) as usize,
            eq_gains: [0.0; 10],
            show_lrc: ini.get_bool("main", "show_lrc", true),
            show_cover: ini.get_bool("main", "show_cover", true),
            show_vis: ini.get_bool("main", "vision", true),
            vis_mode: ini.get_i32("main", "vis_sub", 0),
            show_eq: false,
            show_sidebar: ini.get_bool("main", "list_panel_open", true),
            mini: ini.get_bool("main", "mini_mode", false),
            on_top: ini.get_bool("main", "on_top", false),
            maximized: ini.get_bool("main", "maximized", false),
            fullscreen: ini.get_bool("main", "full_screen", false),
            transparent: ini.get_bool("main", "transparent", false),
            systray: ini.get_bool("main", "on_systray", true),
            include_subdir: ini.get_bool("main", "include_subdir", true),
            save_position: ini.get_bool("main", "save_position", true),
            auto_play: ini.get_bool("main", "auto_play", false),
            taskbar_progress: ini.get_bool("main", "taskbar_progress", true),
            mod_filename: ini.get_bool("main", "mod_filename", false),
            lrc_dir_enabled: ini.get_bool("main", "lrc_dir_enabled", false),
            lrc_dir: ini.get("main", "lrc_dir").unwrap_or_default(),
            exclusive: ini.get_bool("main", "exclusive", false),
            buffer_ms: ini
                .get("main", "buffer")
                .and_then(|s| s.parse().ok())
                .unwrap_or(100)
                .clamp(20, 2000) as u32,
            device_name: ini.get("main", "device_name").unwrap_or_default(),
            devices: Vec::new(),
            hk_global: ini.get_i32("main", "hk_global", 1) != 0,
            hotkeys: default_hotkeys(&ini),
            font_name: ini
                .get("main", "font")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "微软雅黑".into()),
            alpha: ini.get_i32("main", "transparency1_tune", 255).clamp(40, 255) as u8,
            pos: (
                ini.get_i32("main", "pos_x", 120),
                ini.get_i32("main", "pos_y", 80),
                ini.get_i32("main", "pos_w", 880).max(420),
                ini.get_i32("main", "pos_h", 560).max(160),
            ),
            mini_pos: (
                ini.get_i32("main", "pos_mini_x", 120),
                ini.get_i32("main", "pos_mini_y", 80),
            ),
            desk_pos: (
                ini.get_i32("main", "desktop_lrc_x", 80),
                ini.get_i32("main", "desktop_lrc_y", 80),
            ),
            settings_pos: (
                ini.get_i32("main", "pos_form1_x", 160),
                ini.get_i32("main", "pos_form1_y", 120),
            ),
            open_dir: ini.get("main", "open_dir").unwrap_or_default(),
            resume_tenths: ini.get_i32("main", "play_position", 0).max(0),
            use_resume: ini.get_bool("main", "save_position", true),
            now_title: ini.get("main", "custom_name").unwrap_or_else(|| "AirPlay".into()),
            now_sub: String::new(),
            lrc: Vec::new(),
            cover_bytes: None,
            wall_path: None,
            visuals_gen: 1,
            skins: skin::load_all(&exe_dir),
            skin_index: 0,
            note: String::new(),
            edit_mode: EditMode::Hidden,
            desktop_lrc: ini.get_i32("main", "desktop_lrc", 0) != 0,
            desk_lock: false,
            startup_minimized: ini.get_i32("main", "minimized", 0),
            rng: UNIX_EPOCH.elapsed().map(|d| d.as_nanos() as u64).unwrap_or(1),
            restore: None,
            ini,
        };
        if let Some(custom) = app.ini.get("main", "eq_custom") {
            let nums: Vec<f32> = custom.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            for (i, n) in nums.into_iter().take(10).enumerate() {
                app.eq_gains[i] = n.clamp(-15.0, 15.0);
            }
        }
        if app.eq_preset > 0 && app.eq_preset < PRESETS.len() {
            app.eq_gains = PRESETS[app.eq_preset].1;
        }
        apply_font_colors(&mut app);
        app.load_lists();
        let wanted = app.ini.get("main", "m3u").unwrap_or_default();
        if let Some(i) = app
            .lists
            .iter()
            .position(|p| p.path.file_name().and_then(|n| n.to_str()) == Some(wanted.as_str()))
        {
            app.view = i;
        }
        let last = app.ini.get_i32("main", "last_song", -1);
        if last >= 0 {
            app.selected = last as usize;
            if app.selected >= app.tracks().len() {
                app.selected = 0;
            }
        }
        let skin_name = app.ini.get("main", "skin").or_else(|| app.ini.get("main", "wallpaper"));
        if let Some(name) = skin_name {
            let file = Path::new(&name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| name.clone());
            if let Some(i) = app.skins.iter().position(|s| s.name == file || s.name == name) {
                app.skin_index = i;
            }
        }
        app.apply_skin_wall();
        if app.random {
            app.rebuild_shuffle();
        }
        app
    }

    pub fn save(&mut self) {
        let ini = &mut self.ini;
        ini.set("main", "custom_name", &self.custom_name);
        ini.set("main", "open_dir", &self.open_dir);
        if let Some(list) = self.lists.get(self.view) {
            if let Some(name) = list.path.file_name().and_then(|s| s.to_str()) {
                ini.set("main", "m3u", name);
            }
        }
        let last = self.play_index.unwrap_or(self.selected) as i32;
        ini.set_i32("main", "last_song", last);
        ini.set_bool("main", "sort", self.lists.get(self.view).map(|l| l.sort).unwrap_or(true));
        ini.set_bool("main", "random", self.random);
        ini.set_bool("main", "track_play", self.track_play);
        ini.set_bool("main", "repeat", self.repeat_list);
        ini.set_bool("main", "pl_loop", self.pl_loop);
        ini.set_bool("main", "group", self.lists.get(self.view).map(|l| l.group).unwrap_or(false));
        ini.set_bool("main", "show_lrc", self.show_lrc);
        ini.set_bool("main", "show_cover", self.show_cover);
        ini.set_bool("main", "vision", self.show_vis);
        ini.set_i32("main", "vis_sub", self.vis_mode);
        ini.set_bool("main", "mini_mode", self.mini);
        ini.set_bool("main", "on_top", self.on_top);
        ini.set_bool("main", "maximized", self.maximized);
        ini.set_bool("main", "full_screen", self.fullscreen);
        ini.set_bool("main", "transparent", self.transparent);
        ini.set_bool("main", "on_systray", self.systray);
        ini.set_bool("main", "include_subdir", self.include_subdir);
        ini.set_bool("main", "save_position", self.save_position);
        ini.set_bool("main", "auto_play", self.auto_play);
        ini.set_bool("main", "taskbar_progress", self.taskbar_progress);
        ini.set_bool("main", "mod_filename", self.mod_filename);
        ini.set_bool("main", "lrc_dir_enabled", self.lrc_dir_enabled);
        ini.set("main", "lrc_dir", &self.lrc_dir);
        ini.set_bool("main", "exclusive", self.exclusive);
        ini.set("main", "buffer", &self.buffer_ms.to_string());
        ini.set("main", "device_name", &self.device_name);
        ini.set_i32("main", "device", 0);
        ini.set_i32("main", "volume_tune", self.volume);
        ini.set_bool("main", "eq_on", self.eq_on);
        ini.set_i32("main", "eq_preset", self.eq_preset as i32);
        let custom = self
            .eq_gains
            .iter()
            .map(|g| format!("{}", g.round() as i32))
            .collect::<Vec<_>>()
            .join(",");
        ini.set("main", "eq_custom", &custom);
        ini.set_i32("main", "hk_global", if self.hk_global { 1 } else { 0 });
        const HK: [&str; 12] = [
            "hk_play",
            "hk_stop",
            "hk_prev",
            "hk_next",
            "hk_volume+",
            "hk_volume-",
            "hk_mute",
            "hk_minimode",
            "hk_minimize",
            "hk_exit",
            "hk_random",
            "hk_track_play",
        ];
        for (key, val) in HK.iter().zip(self.hotkeys.iter()) {
            ini.set("main", key, val);
        }
        ini.set("main", "font", &self.font_name);
        ini.set_i32("main", "transparency1_tune", self.alpha as i32);
        ini.set_i32("main", "pos_x", self.pos.0);
        ini.set_i32("main", "pos_y", self.pos.1);
        ini.set_i32("main", "pos_w", self.pos.2);
        ini.set_i32("main", "pos_h", self.pos.3);
        ini.set_i32("main", "pos_mini_x", self.mini_pos.0);
        ini.set_i32("main", "pos_mini_y", self.mini_pos.1);
        ini.set_i32("main", "desktop_lrc_x", self.desk_pos.0);
        ini.set_i32("main", "desktop_lrc_y", self.desk_pos.1);
        ini.set_i32("main", "desktop_lrc", if self.desktop_lrc { 1 } else { 0 });
        ini.set_i32("main", "pos_form1_x", self.settings_pos.0);
        ini.set_i32("main", "pos_form1_y", self.settings_pos.1);
        ini.set_bool("main", "list_panel_open", self.show_sidebar);
        if let Some(skin) = self.skins.get(self.skin_index) {
            ini.set("main", "skin", &skin.name);
            if let Some(img) = &skin.image {
                if let Some(name) = img.file_name().and_then(|s| s.to_str()) {
                    ini.set("main", "wallpaper", name);
                }
            }
        }
        if self.save_position {
            ini.set_i32("main", "play_position", self.resume_tenths.max(0));
        }
        if self.random && !self.shuffle.is_empty() {
            let q = self
                .shuffle
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",");
            ini.set("main", "play_queue", &q);
        }
        let _ = self.ini.save(&self.ini_path);
        for list in &self.lists {
            list.save(&self.exe_dir);
        }
    }

    pub fn tracks(&self) -> &[Track] {
        self.lists.get(self.view).map(|l| l.tracks.as_slice()).unwrap_or(&[])
    }

    pub fn skin(&self) -> &Skin {
        &self.skins[self.skin_index.min(self.skins.len().saturating_sub(1))]
    }

    pub fn volume_linear(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume.clamp(0, 100) as f32 / 100.0
        }
    }

    fn load_lists(&mut self) {
        let dir = self.exe_dir.join("playlists");
        let _ = std::fs::create_dir_all(&dir);
        let mut files = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for ent in rd.flatten() {
                let path = ent.path();
                if path.extension().and_then(|e| e.to_str()).map(|e| {
                    e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("m3u8")
                }) == Some(true)
                {
                    files.push(path);
                }
            }
        }
        files.sort();
        self.lists = files
            .iter()
            .map(|p| Playlist::load(p, &self.exe_dir))
            .collect();
        if self.lists.is_empty() {
            let path = dir.join("默认列表.m3u8");
            let pl = Playlist {
                name: "默认列表".into(),
                path,
                tracks: Vec::new(),
                sort: true,
                group: false,
            };
            pl.save(&self.exe_dir);
            self.lists.push(pl);
        }
    }

    pub fn add_paths(&mut self, paths: &[PathBuf]) -> Option<usize> {
        let mut first = None;
        for path in paths {
            if is_url(&path.to_string_lossy()) {
                let loc = path.to_string_lossy().to_string();
                self.push_track(Track {
                    location: loc.clone(),
                    title: loc,
                    start: 0.0,
                    end: None,
                    is_group: false,
                    missing: false,
                });
                if first.is_none() {
                    first = Some(self.tracks().len().saturating_sub(1));
                }
                continue;
            }
            if path.is_dir() {
                self.add_folder(path);
                if first.is_none() {
                    first = Some(self.tracks().len().saturating_sub(1));
                }
                continue;
            }
            let before = self.tracks().len();
            self.import_file(path);
            if self.tracks().len() > before && first.is_none() {
                first = Some(before);
            }
        }
        self.finish_edit();
        first
    }

    pub fn add_folder(&mut self, dir: &Path) {
        let mut files = Vec::new();
        collect_audio(dir, self.include_subdir, &mut files);
        files.sort();
        let grouped = self.lists.get(self.view).map(|l| l.group).unwrap_or(false);
        let mut last_dir = None;
        for file in files {
            if grouped {
                let parent = file.parent().map(|p| p.to_path_buf());
                if parent != last_dir {
                    if let Some(name) = parent.as_ref().and_then(|p| p.file_name()) {
                        self.push_track(Track::group(name.to_string_lossy().to_string()));
                    }
                    last_dir = parent;
                }
            }
            self.import_file(&file);
        }
    }

    fn import_file(&mut self, path: &Path) {
        let ext = playlist::ext_of(path);
        if ext.as_deref() == Some("cue") {
            if let Some(text) = read_text(path) {
                let mut tracks = parse_cue(&text, path);
                for t in &mut tracks {
                    t.missing = !Path::new(&t.location).exists();
                }
                for t in tracks {
                    self.push_track(t);
                }
            }
            return;
        }
        if matches!(ext.as_deref(), Some("m3u") | Some("m3u8")) {
            if let Some(text) = read_text(path) {
                let mut sort = false;
                let mut group = false;
                let tracks = playlist::parse_m3u(&text, path, &self.exe_dir, &mut sort, &mut group);
                for t in tracks {
                    self.push_track(t);
                }
            }
            return;
        }
        if !is_audio_file(path) && !is_list_file(path) {
            return;
        }
        let loc = path.to_string_lossy().to_string();
        self.push_track(Track {
            title: stem_title(&loc),
            location: loc,
            start: 0.0,
            end: None,
            is_group: false,
            missing: !path.exists(),
        });
    }

    fn push_track(&mut self, track: Track) {
        let exe = self.exe_dir.clone();
        if let Some(list) = self.lists.get_mut(self.view) {
            list.tracks.push(track);
            list.save(&exe);
        }
    }

    fn finish_edit(&mut self) {
        let sort = self.lists.get(self.view).map(|l| l.sort).unwrap_or(false);
        if sort {
            self.sort_view();
        }
        if self.random {
            self.rebuild_shuffle();
        }
    }

    pub fn sort_view(&mut self) {
        let current = self.current_key();
        if let Some(list) = self.lists.get_mut(self.view) {
            let groups = list.group;
            if groups {
                list.tracks.sort_by(|a, b| {
                    a.title.to_lowercase().cmp(&b.title.to_lowercase())
                });
            } else {
                list.tracks
                    .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            list.sort = true;
        }
        self.restore_key(current);
        let exe = self.exe_dir.clone();
        if let Some(list) = self.lists.get(self.view) {
            list.save(&exe);
        }
    }

    pub fn toggle_sort(&mut self) {
        if let Some(list) = self.lists.get_mut(self.view) {
            list.sort = !list.sort;
            if list.sort {
                // sorted below
            }
        }
        if self.lists.get(self.view).map(|l| l.sort).unwrap_or(false) {
            self.sort_view();
        }
    }

    pub fn toggle_group(&mut self) {
        if let Some(list) = self.lists.get_mut(self.view) {
            list.group = !list.group;
        }
    }

    pub fn new_list(&mut self) {
        let dir = self.exe_dir.join("playlists");
        let _ = std::fs::create_dir_all(&dir);
        let mut name = "新列表".to_string();
        let mut i = 2;
        while dir.join(format!("{name}.m3u8")).exists() {
            name = format!("新列表{i}");
            i += 1;
        }
        let path = dir.join(format!("{name}.m3u8"));
        let pl = Playlist {
            name: name.clone(),
            path,
            tracks: Vec::new(),
            sort: true,
            group: false,
        };
        pl.save(&self.exe_dir);
        self.lists.push(pl);
        self.view = self.lists.len() - 1;
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn delete_view_list(&mut self) {
        if self.lists.len() <= 1 {
            self.note = "至少保留一个列表".into();
            return;
        }
        let path = self.lists[self.view].path.clone();
        let _ = std::fs::remove_file(path);
        let removed = self.view;
        self.lists.remove(removed);
        if self.play_list == Some(removed) {
            self.play_list = None;
            self.play_index = None;
        } else if let Some(p) = self.play_list {
            if p > removed {
                self.play_list = Some(p - 1);
            }
        }
        if self.view >= self.lists.len() {
            self.view = self.lists.len() - 1;
        }
        self.selected = 0;
    }

    pub fn switch_list(&mut self, index: usize) {
        if index < self.lists.len() {
            self.view = index;
            self.selected = 0;
            self.scroll = 0;
        }
    }

    pub fn remove_selected(&mut self) -> Option<PathBuf> {
        let idx = self.selected;
        let track = self.tracks().get(idx).cloned();
        let Some(track) = track else { return None };
        if let Some(list) = self.lists.get_mut(self.view) {
            if idx < list.tracks.len() {
                list.tracks.remove(idx);
            }
        }
        if self.play_list == Some(self.view) && self.play_index == Some(idx) {
            self.play_index = None;
        } else if self.play_list == Some(self.view) {
            if let Some(p) = self.play_index {
                if p > idx {
                    self.play_index = Some(p - 1);
                }
            }
        }
        if self.selected > 0 && self.selected >= self.tracks().len() {
            self.selected = self.tracks().len().saturating_sub(1);
        }
        let exe = self.exe_dir.clone();
        if let Some(list) = self.lists.get(self.view) {
            list.save(&exe);
        }
        if self.random {
            self.rebuild_shuffle();
        }
        if track.playable() && !is_url(&track.location) {
            Some(PathBuf::from(track.location))
        } else {
            None
        }
    }

    pub fn clear_view(&mut self) {
        let exe = self.exe_dir.clone();
        if let Some(list) = self.lists.get_mut(self.view) {
            list.tracks.clear();
            list.save(&exe);
        }
        if self.play_list == Some(self.view) {
            self.play_index = None;
        }
        self.selected = 0;
        self.shuffle.clear();
    }

    pub fn find_next(&mut self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return false;
        }
        let n = self.tracks().len();
        if n == 0 {
            return false;
        }
        let start = self.selected.saturating_add(1) % n;
        for step in 0..n {
            let i = (start + step) % n;
            let t = &self.tracks()[i];
            if t.title.to_lowercase().contains(&q) || t.location.to_lowercase().contains(&q) {
                self.selected = i;
                return true;
            }
        }
        false
    }

    pub fn locate_selected(&self) -> Option<PathBuf> {
        let t = self.tracks().get(self.selected)?;
        if t.playable() && !is_url(&t.location) {
            Some(PathBuf::from(&t.location))
        } else {
            None
        }
    }

    pub fn cycle_skin(&mut self) {
        if self.skins.is_empty() {
            return;
        }
        self.skin_index = (self.skin_index + 1) % self.skins.len();
        self.apply_skin_wall();
    }

    fn apply_skin_wall(&mut self) {
        self.wall_path = self.skins.get(self.skin_index).and_then(|s| s.image.clone());
        self.visuals_gen = self.visuals_gen.wrapping_add(1);
    }

    pub fn cycle_device(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        let idx = self
            .devices
            .iter()
            .position(|d| d == &self.device_name)
            .unwrap_or(0);
        self.device_name = self.devices[(idx + 1) % self.devices.len()].clone();
    }

    pub fn cycle_buffer(&mut self) {
        self.buffer_ms = match self.buffer_ms {
            0..=50 => 100,
            51..=100 => 200,
            101..=200 => 400,
            201..=400 => 800,
            _ => 50,
        };
    }

    pub fn prepare_play(&mut self, index: usize, resume: bool) -> Option<PlayReq> {
        if index >= self.tracks().len() || !self.tracks()[index].playable() {
            if let Some(next) = self.next_playable(index) {
                return self.prepare_play(next, resume);
            }
            self.note = "没有可播放的曲目".into();
            return None;
        }
        self.selected = index;
        self.play_list = Some(self.view);
        self.play_index = Some(index);
        let track = self.tracks()[index].clone();
        let mut start = track.start;
        let end = track.end;
        if resume && self.use_resume && self.resume_tenths > 0 {
            let extra = self.resume_tenths as f64 / 10.0;
            if end.map(|e| start + extra < e - 0.2).unwrap_or(true) {
                start += extra;
            }
            self.use_resume = false;
        }
        self.load_track_text(&track);
        if self.random {
            if let Some(pos) = self.shuffle.iter().position(|i| *i == index) {
                self.shuffle_pos = pos;
            }
        }
        Some(PlayReq {
            url: is_url(&track.location),
            location: track.location,
            start,
            end,
        })
    }

    fn load_track_text(&mut self, track: &Track) {
        let fallback = track.title.clone();
        let meta = if is_url(&track.location) {
            Meta::default()
        } else {
            meta::read_meta(Path::new(&track.location))
        };
        self.now_title = display_now(&meta, &fallback);
        self.now_sub = if meta.album.is_empty() {
            String::new()
        } else {
            meta.album.clone()
        };
        if self.mod_filename && !meta.title.is_empty() {
            let title = self.now_title.clone();
            if let Some(list) = self.play_index.and_then(|i| {
                let view = self.view;
                self.lists.get_mut(view).and_then(|l| l.tracks.get_mut(i))
            }) {
                list.title = title;
            }
        }
        self.lrc = self.find_lrc(&meta, track);
        self.cover_bytes = if self.show_cover { meta.cover } else { None };
        self.visuals_gen = self.visuals_gen.wrapping_add(1);
        self.note.clear();
    }

    fn find_lrc(&self, meta: &Meta, track: &Track) -> Vec<LrcLine> {
        if !self.show_lrc {
            return Vec::new();
        }
        if meta.lyrics.contains('[') {
            let lines = lrc::parse(&meta.lyrics);
            if !lines.is_empty() {
                return lines;
            }
        }
        if is_url(&track.location) {
            return Vec::new();
        }
        let path = Path::new(&track.location);
        let mut candidates = Vec::new();
        if let Some(parent) = path.parent() {
            if let Some(stem) = path.file_stem() {
                candidates.push(parent.join(format!("{}.lrc", stem.to_string_lossy())));
            }
            if !meta.artist.is_empty() && !meta.title.is_empty() {
                candidates.push(parent.join(format!("{} - {}.lrc", meta.artist, meta.title)));
            }
        }
        if self.lrc_dir_enabled {
            let dir = if self.lrc_dir.is_empty() {
                self.exe_dir.join("plugins").join("lyrics")
            } else {
                PathBuf::from(&self.lrc_dir)
            };
            if let Some(stem) = path.file_stem() {
                candidates.push(dir.join(format!("{}.lrc", stem.to_string_lossy())));
            }
            if !meta.artist.is_empty() && !meta.title.is_empty() {
                candidates.push(dir.join(format!("{} - {}.lrc", meta.artist, meta.title)));
            }
        }
        for c in candidates {
            if let Some(text) = read_text(&c) {
                let lines = lrc::parse(&text);
                if !lines.is_empty() {
                    return lines;
                }
            }
        }
        Vec::new()
    }

    pub fn lrc_at(&self, seconds: f64) -> (String, String) {
        let ms = (seconds * 1000.0) as i64;
        let Some((i, line)) = lrc::current(&self.lrc, ms) else {
            return (String::new(), String::new());
        };
        let next = self
            .lrc
            .get(i + 1)
            .map(|l| l.text.clone())
            .unwrap_or_default();
        (line.text.clone(), next)
    }

    pub fn auto_next(&mut self) -> Option<usize> {
        if self.track_play {
            return self.play_index.or(Some(self.selected)).filter(|i| {
                self.lists
                    .get(self.view)
                    .and_then(|l| l.tracks.get(*i))
                    .map(|t| t.playable())
                    .unwrap_or(false)
            });
        }
        self.step(1, true)
    }

    pub fn manual_next(&mut self) -> Option<usize> {
        self.step(1, false)
    }

    pub fn manual_prev(&mut self) -> Option<usize> {
        self.step(-1, false)
    }

    fn step(&mut self, dir: i32, from_auto: bool) -> Option<usize> {
        let list = self.play_list.unwrap_or(self.view);
        if list != self.view {
            self.view = list;
        }
        let n = self.tracks().len();
        if n == 0 {
            return None;
        }
        if self.random {
            if self.shuffle.is_empty() {
                self.rebuild_shuffle();
            }
            if self.shuffle.is_empty() {
                return None;
            }
            if dir < 0 {
                if self.shuffle_pos == 0 {
                    if self.repeat_list {
                        self.shuffle_pos = self.shuffle.len() - 1;
                    } else {
                        return None;
                    }
                } else {
                    self.shuffle_pos -= 1;
                }
            } else if self.shuffle_pos + 1 < self.shuffle.len() {
                self.shuffle_pos += 1;
            } else if self.repeat_list {
                self.rebuild_shuffle();
                self.shuffle_pos = 0;
            } else if from_auto && self.pl_loop {
                return self.switch_playlist(1);
            } else {
                return None;
            }
            return self.shuffle.get(self.shuffle_pos).copied();
        }
        let cur = self.play_index.unwrap_or(self.selected);
        let mut i = cur as i32;
        for _ in 0..n {
            i += dir;
            if i < 0 || i >= n as i32 {
                break;
            }
            if self.tracks()[i as usize].playable() {
                return Some(i as usize);
            }
        }
        if dir > 0 && (self.repeat_list || (from_auto && self.pl_loop && !self.repeat_list)) {
            if self.repeat_list {
                return self.next_playable(0).or_else(|| {
                    if self.tracks().first().map(|t| t.playable()).unwrap_or(false) {
                        Some(0)
                    } else {
                        None
                    }
                });
            }
            return self.switch_playlist(1);
        }
        if dir < 0 && self.repeat_list {
            return self.last_playable();
        }
        None
    }

    fn switch_playlist(&mut self, dir: i32) -> Option<usize> {
        if self.lists.is_empty() {
            return None;
        }
        let start = self.play_list.unwrap_or(self.view) as i32;
        for step in 1..=self.lists.len() as i32 {
            let idx = (start + dir * step).rem_euclid(self.lists.len() as i32) as usize;
            self.view = idx;
            self.play_list = Some(idx);
            if let Some(i) = self.next_playable(0) {
                return Some(i);
            }
            if self.tracks().first().map(|t| t.playable()).unwrap_or(false) {
                return Some(0);
            }
        }
        None
    }

    fn next_playable(&self, from: usize) -> Option<usize> {
        self.tracks()
            .iter()
            .enumerate()
            .skip(from)
            .find(|(_, t)| t.playable())
            .map(|(i, _)| i)
    }

    fn last_playable(&self) -> Option<usize> {
        self.tracks()
            .iter()
            .enumerate()
            .rev()
            .find(|(_, t)| t.playable())
            .map(|(i, _)| i)
    }

    pub fn rebuild_shuffle(&mut self) {
        self.shuffle = self
            .tracks()
            .iter()
            .enumerate()
            .filter(|(_, t)| t.playable())
            .map(|(i, _)| i)
            .collect();
        let mut state = self.rng | 1;
        for i in (1..self.shuffle.len()).rev() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let j = (state as usize) % (i + 1);
            self.shuffle.swap(i, j);
        }
        self.rng = state;
        self.shuffle_pos = self
            .play_index
            .and_then(|cur| self.shuffle.iter().position(|i| *i == cur))
            .unwrap_or(0);
    }

    pub fn set_eq_band(&mut self, band: usize, gain: f32) {
        if band < 10 {
            self.eq_gains[band] = gain.clamp(-15.0, 15.0);
            self.eq_preset = 0;
            self.eq_on = true;
        }
    }

    pub fn cycle_preset(&mut self) {
        self.eq_preset = (self.eq_preset + 1) % PRESETS.len();
        self.eq_gains = PRESETS[self.eq_preset].1;
        self.eq_on = self.eq_preset != 0 || self.eq_gains.iter().any(|g| g.abs() > 0.05);
    }

    pub fn note_pos(&mut self, seconds: f64) {
        if self.save_position {
            self.resume_tenths = (seconds * 10.0).round() as i32;
        }
    }

    fn current_key(&self) -> Option<(String, f64)> {
        let i = if self.play_list == Some(self.view) {
            self.play_index
        } else {
            None
        }?;
        let t = self.tracks().get(i)?;
        Some((t.location.clone(), t.start))
    }

    fn restore_key(&mut self, key: Option<(String, f64)>) {
        let Some((loc, start)) = key else { return };
        if let Some(i) = self.tracks().iter().position(|t| t.location == loc && (t.start - start).abs() < 0.01)
        {
            if self.play_list == Some(self.view) {
                self.play_index = Some(i);
            }
            self.selected = i;
        }
    }
}

fn default_hotkeys(ini: &Ini) -> [String; 12] {
    let keys = [
        ("hk_play", "Ctrl+Alt+Space"),
        ("hk_stop", "Ctrl+Alt+S"),
        ("hk_prev", "Ctrl+Alt+Left"),
        ("hk_next", "Ctrl+Alt+Right"),
        ("hk_volume+", "Ctrl+Alt+Up"),
        ("hk_volume-", "Ctrl+Alt+Down"),
        ("hk_mute", "Ctrl+Alt+M"),
        ("hk_minimode", "Ctrl+Alt+N"),
        ("hk_minimize", "Ctrl+Alt+H"),
        ("hk_exit", "Ctrl+Alt+Q"),
        ("hk_random", "Ctrl+Alt+R"),
        ("hk_track_play", "Ctrl+Alt+T"),
    ];
    std::array::from_fn(|i| {
        let (k, default) = keys[i];
        match ini.get("main", k) {
            Some(s) => {
                let head = s.split('|').next().unwrap_or("").trim();
                if head.is_empty() || head == "0" {
                    String::new()
                } else {
                    head.to_string()
                }
            }
            None => default.into(),
        }
    })
}

fn apply_font_colors(app: &mut App) {
    let ini = &app.ini;
    if let Some(bg) = ini.get("main", "fontcolor1_back").and_then(|s| s.parse::<i64>().ok()).and_then(colorref_rgb)
    {
        if let Some(skin) = app.skins.get_mut(app.skin_index) {
            skin.bg = bg;
        }
    }
    if let Some(text) = ini
        .get("main", "fontcolor1_fore")
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(colorref_rgb)
    {
        if let Some(skin) = app.skins.get_mut(app.skin_index) {
            skin.text = text;
        }
    }
}

fn collect_audio(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for ent in rd.flatten() {
        let path = ent.path();
        if path.is_dir() {
            if recursive {
                collect_audio(&path, true, out);
            }
        } else if is_audio_file(&path) || playlist::ext_of(&path).as_deref() == Some("cue") {
            out.push(path);
        }
    }
}
