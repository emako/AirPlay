use crate::config::Ini;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Skin {
    pub name: String,
    pub bg: [u8; 3],
    pub panel: [u8; 3],
    pub text: [u8; 3],
    pub dim: [u8; 3],
    pub accent: [u8; 3],
    pub playing: [u8; 3],
    pub select: [u8; 3],
    pub progress: [u8; 3],
    pub image: Option<PathBuf>,
    pub design: (i32, i32),
    pub zones: HashMap<String, (i32, i32, i32, i32)>,
}

impl Skin {
    pub fn solid(name: &str, bg: [u8; 3], panel: [u8; 3], accent: [u8; 3], text: [u8; 3]) -> Self {
        Self {
            name: name.into(),
            bg,
            panel,
            text,
            dim: mix(text, bg, 0.45),
            accent,
            playing: accent,
            select: mix(accent, panel, 0.35),
            progress: accent,
            image: None,
            design: (800, 600),
            zones: HashMap::new(),
        }
    }
}

pub fn builtins() -> Vec<Skin> {
    vec![
        Skin::solid(
            "暗夜",
            [22, 24, 31],
            [14, 16, 22],
            [90, 168, 250],
            [232, 234, 237],
        ),
        Skin::solid(
            "海蓝",
            [18, 40, 58],
            [12, 28, 42],
            [120, 198, 214],
            [226, 236, 240],
        ),
        Skin::solid(
            "原木",
            [54, 42, 34],
            [38, 30, 24],
            [214, 164, 96],
            [244, 232, 214],
        ),
        Skin::solid(
            "纸白",
            [236, 238, 241],
            [248, 249, 250],
            [36, 99, 168],
            [32, 36, 42],
        ),
    ]
}

pub fn load_all(exe_dir: &Path) -> Vec<Skin> {
    let mut skins = builtins();
    let skin_root = exe_dir.join("skins");
    let _ = fs::create_dir_all(&skin_root);
    ensure_examples(&skin_root);
    if let Ok(rd) = fs::read_dir(&skin_root) {
        let mut dirs: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        dirs.sort();
        for dir in dirs {
            if dir.is_dir() {
                if let Some(skin) = load_dir(&dir) {
                    if let Some(slot) = skins.iter_mut().find(|s| s.name == skin.name) {
                        *slot = skin;
                    } else {
                        skins.push(skin);
                    }
                }
            }
        }
    }
    let wall = exe_dir.join("plugins").join("wallpaper");
    if let Ok(rd) = fs::read_dir(&wall) {
        let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        for file in files {
            if is_image(&file) {
                let name = file
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "壁纸".into());
                let mut skin = Skin::solid(
                    &name,
                    [20, 22, 28],
                    [16, 18, 24],
                    [120, 180, 230],
                    [240, 242, 245],
                );
                skin.image = Some(file);
                skins.push(skin);
            }
        }
    }
    skins
}

fn ensure_examples(root: &Path) {
    for skin in builtins() {
        let dir = root.join(&skin.name);
        let ini_path = dir.join("skin.ini");
        if ini_path.exists() {
            continue;
        }
        let _ = fs::create_dir_all(&dir);
        let mut ini = Ini::default();
        ini.set("skin", "name", &skin.name);
        ini.set("skin", "bg", &hex(skin.bg));
        ini.set("skin", "panel", &hex(skin.panel));
        ini.set("skin", "text", &hex(skin.text));
        ini.set("skin", "dim", &hex(skin.dim));
        ini.set("skin", "accent", &hex(skin.accent));
        ini.set("skin", "playing", &hex(skin.playing));
        ini.set("skin", "select", &hex(skin.select));
        ini.set("skin", "progress", &hex(skin.progress));
        ini.set("zones", "design_w", "800");
        ini.set("zones", "design_h", "600");
        let _ = ini.save(&ini_path);
    }
}

fn load_dir(dir: &Path) -> Option<Skin> {
    let ini = Ini::load(&dir.join("skin.ini"));
    let name = ini
        .get("skin", "name")
        .or_else(|| {
            dir.file_name()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "皮肤".into());
    let mut skin = Skin::solid(&name, [22, 24, 31], [14, 16, 22], [90, 168, 250], [232, 234, 237]);
    skin.bg = color_or(&ini, "bg", skin.bg);
    skin.panel = color_or(&ini, "panel", skin.panel);
    skin.text = color_or(&ini, "text", skin.text);
    skin.dim = color_or(&ini, "dim", skin.dim);
    skin.accent = color_or(&ini, "accent", skin.accent);
    skin.playing = color_or(&ini, "playing", skin.playing);
    skin.select = color_or(&ini, "select", skin.select);
    skin.progress = color_or(&ini, "progress", skin.progress);
    skin.design = (
        ini.get_i32("zones", "design_w", 800).max(1),
        ini.get_i32("zones", "design_h", 600).max(1),
    );
    for key in ["close", "min", "max", "prev", "play", "next", "progress", "volume", "list"] {
        if let Some(v) = ini.get("zones", key) {
            if let Some(rect) = parse_zone(&v) {
                skin.zones.insert(key.into(), rect);
            }
        }
    }
    for name in ["background.png", "background.jpg", "skin.png", "skin.jpg", "bg.png", "bg.jpg"] {
        let p = dir.join(name);
        if p.exists() {
            skin.image = Some(p);
            break;
        }
    }
    Some(skin)
}

fn color_or(ini: &Ini, key: &str, default: [u8; 3]) -> [u8; 3] {
    ini.get("skin", key)
        .as_deref()
        .and_then(parse_hex)
        .unwrap_or(default)
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

fn parse_zone(s: &str) -> Option<(i32, i32, i32, i32)> {
    let p: Vec<_> = s.split(',').map(|x| x.trim().parse::<i32>()).collect();
    if p.len() != 4 || p.iter().any(|n| n.is_err()) {
        return None;
    }
    Some((p[0].clone().unwrap(), p[1].clone().unwrap(), p[2].clone().unwrap(), p[3].clone().unwrap()))
}

fn hex(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("bmp") | Some("jpg") | Some("jpeg") | Some("png") | Some("gif")
    )
}

pub fn colorref_rgb(c: i64) -> Option<[u8; 3]> {
    if !(0..=0x00FF_FFFF).contains(&c) {
        return None;
    }
    let c = c as u32;
    Some([(c & 255) as u8, ((c >> 8) & 255) as u8, ((c >> 16) & 255) as u8])
}
