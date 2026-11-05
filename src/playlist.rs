use crate::cue::parse_cue;
use crate::textutil::{is_url, read_text, stem_title};
use std::path::{Path, PathBuf};

pub use crate::track::Track;

#[derive(Clone, Debug)]
pub struct Playlist {
    pub name: String,
    pub path: PathBuf,
    pub tracks: Vec<Track>,
    pub sort: bool,
    pub group: bool,
}

impl Playlist {
    pub fn load(path: &Path, exe_dir: &Path) -> Self {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "列表".into());
        let mut pl = Self {
            name,
            path: path.to_path_buf(),
            tracks: Vec::new(),
            sort: false,
            group: false,
        };
        let Some(text) = read_text(path) else {
            return pl;
        };
        pl.tracks = parse_m3u(&text, path, exe_dir, &mut pl.sort, &mut pl.group);
        pl
    }

    pub fn save(&self, exe_dir: &Path) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut body = String::from("\u{feff}");
        match (self.sort, self.group) {
            (true, true) => body.push_str("#EXTM3U Group Sort\r\n"),
            (false, true) => body.push_str("#EXTM3U Group\r\n"),
            (true, false) => body.push_str("#EXTM3U Sort\r\n"),
            (false, false) => body.push_str("#EXTM3U\r\n"),
        }
        for t in &self.tracks {
            if t.is_group {
                body.push_str("#EXTGRP:");
                body.push_str(&t.title);
                body.push_str("\r\n");
                continue;
            }
            body.push_str("#EXTINF:-1,");
            body.push_str(&t.title.replace(['\r', '\n'], " "));
            body.push_str("\r\n");
            body.push_str(&store_location(exe_dir, t));
            body.push_str("\r\n");
        }
        let _ = std::fs::write(&self.path, body);
    }
}

pub fn parse_m3u(
    text: &str,
    list_path: &Path,
    exe_dir: &Path,
    sort: &mut bool,
    group: &mut bool,
) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut pending_title: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("#EXTM3U") {
                if upper.contains("SORT") {
                    *sort = true;
                }
                if upper.contains("GROUP") {
                    *group = true;
                }
            } else if let Some(name) = line.strip_prefix("#EXTGRP:") {
                tracks.push(Track::group(name.trim()));
            } else if let Some(rest) = line.get(8..).filter(|_| upper.starts_with("#EXTINF:")) {
                if let Some((_, title)) = rest.split_once(',') {
                    let title = title.trim();
                    if !title.is_empty() {
                        pending_title = Some(title.to_string());
                    }
                }
            }
            continue;
        }
        if is_cue_path(line) {
            let resolved = resolve_location(exe_dir, list_path, line);
            if let Some(text) = read_text(Path::new(&resolved)) {
                let mut cue_tracks = parse_cue(&text, Path::new(&resolved));
                for t in &mut cue_tracks {
                    t.missing = !is_url(&t.location) && !Path::new(&t.location).exists();
                }
                tracks.extend(cue_tracks);
            }
            pending_title = None;
            continue;
        }
        let (loc, start, end) = split_pipe(line);
        let resolved = if is_url(&loc) {
            loc
        } else {
            resolve_location(exe_dir, list_path, &loc)
        };
        let title = pending_title.take().unwrap_or_else(|| stem_title(&resolved));
        let missing = !is_url(&resolved) && !Path::new(&resolved).exists();
        tracks.push(Track {
            location: resolved,
            title,
            start,
            end,
            is_group: false,
            missing,
        });
    }
    tracks
}

fn is_cue_path(line: &str) -> bool {
    let loc = line.split('|').next().unwrap_or(line);
    Path::new(loc)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("cue"))
        .unwrap_or(false)
}

fn split_pipe(line: &str) -> (String, f64, Option<f64>) {
    let mut parts = line.split('|');
    let loc = parts.next().unwrap_or(line).trim().to_string();
    let start = match parts.next() {
        Some(s) => match s.trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) => return (line.trim().to_string(), 0.0, None),
        },
        None => return (loc, 0.0, None),
    };
    let end = parts.next().and_then(|s| {
        let s = s.trim();
        if s.is_empty() {
            None
        } else {
            s.parse().ok()
        }
    });
    (loc, start, end)
}

pub fn resolve_location(exe_dir: &Path, list_path: &Path, loc: &str) -> String {
    let p = PathBuf::from(loc.trim());
    if p.is_absolute() || is_url(loc) {
        return p.to_string_lossy().to_string();
    }
    let from_exe = exe_dir.join(&p);
    if from_exe.exists() {
        return from_exe.to_string_lossy().to_string();
    }
    if let Some(parent) = list_path.parent() {
        let from_list = parent.join(&p);
        if from_list.exists() {
            return from_list.to_string_lossy().to_string();
        }
    }
    from_exe.to_string_lossy().to_string()
}

fn store_location(exe_dir: &Path, t: &Track) -> String {
    let loc = if is_url(&t.location) {
        t.location.clone()
    } else {
        let path = Path::new(&t.location);
        match path.strip_prefix(exe_dir) {
            Ok(rel) => rel.to_string_lossy().replace('/', "\\"),
            Err(_) => t.location.clone(),
        }
    };
    if t.start > 0.001 || t.end.is_some() {
        match t.end {
            Some(end) => format!("{loc}|{:.3}|{:.3}", t.start, end),
            None => format!("{loc}|{:.3}|", t.start),
        }
    } else {
        loc
    }
}

pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "oga", "opus", "m4a", "m4b", "aac", "mp4", "aiff", "aif", "aifc",
    "alac", "caf", "wma", "ape", "wv", "tta", "mpc", "tak", "dsf", "dff", "mid", "midi",
];

pub fn ext_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

pub fn is_audio_file(path: &Path) -> bool {
    ext_of(path)
        .map(|e| AUDIO_EXTS.iter().any(|x| *x == e))
        .unwrap_or(false)
}

pub fn is_list_file(path: &Path) -> bool {
    matches!(
        ext_of(path).as_deref(),
        Some("m3u") | Some("m3u8") | Some("cue")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_extinf_and_pipe() {
        let text = "#EXTM3U Sort\r\n#EXTINF:-1,歌名\r\nmusic\\a.mp3|1.5|9\r\n#EXTGRP:目录\r\n";
        let mut sort = false;
        let mut group = false;
        let tracks = parse_m3u(
            text,
            Path::new("D:/app/playlists/默认.m3u8"),
            Path::new("D:/app"),
            &mut sort,
            &mut group,
        );
        assert!(sort);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "歌名");
        assert!((tracks[0].start - 1.5).abs() < 0.001);
        assert_eq!(tracks[0].end, Some(9.0));
        assert!(tracks[1].is_group);
    }
}
