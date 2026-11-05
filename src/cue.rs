use crate::track::Track;
use std::path::{Path, PathBuf};

pub fn parse_cue(text: &str, cue_path: &Path) -> Vec<Track> {
    let dir = cue_path.parent().unwrap_or(Path::new("."));
    let mut album_performer = String::new();
    let mut file = String::new();
    let mut number = 0u32;
    let mut title = String::new();
    let mut performer = String::new();
    let mut start: Option<f64> = None;
    let mut in_track = false;
    let mut pending: Vec<Track> = Vec::new();
    let mut out = Vec::new();

    let flush = |pending: &mut Vec<Track>, out: &mut Vec<Track>| {
        if pending.is_empty() {
            return;
        }
        for i in 0..pending.len().saturating_sub(1) {
            let next = pending[i + 1].start;
            if next > pending[i].start {
                pending[i].end = Some(next);
            }
        }
        out.append(pending);
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.to_ascii_uppercase().starts_with("REM ") {
            continue;
        }
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("FILE ") {
            flush(&mut pending, &mut out);
            file = resolve_file(dir, &unquote(&line[5..]));
            in_track = false;
        } else if upper.starts_with("TRACK ") {
            if in_track && start.is_some() && !file.is_empty() {
                pending.push(make_track(
                    &file,
                    number,
                    &title,
                    &performer,
                    &album_performer,
                    start.unwrap_or(0.0),
                ));
            }
            in_track = true;
            number = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(number + 1);
            title.clear();
            performer.clear();
            start = None;
        } else if upper.starts_with("TITLE ") {
            let value = unquote(&line[6..]);
            if in_track {
                title = value;
            }
        } else if upper.starts_with("PERFORMER ") {
            let value = unquote(&line[10..]);
            if in_track {
                performer = value;
            } else {
                album_performer = value;
            }
        } else if upper.starts_with("INDEX ") {
            let mut bits = line.split_whitespace();
            let _ = bits.next();
            let which = bits.next().unwrap_or("");
            let ts = bits.next().unwrap_or("");
            if which == "01" {
                if let Some(sec) = index_seconds(ts) {
                    start = Some(sec);
                }
            } else if which == "00" && start.is_none() {
                start = index_seconds(ts);
            }
        }
    }
    if in_track && !file.is_empty() {
        pending.push(make_track(
            &file,
            number,
            &title,
            &performer,
            &album_performer,
            start.unwrap_or(0.0),
        ));
    }
    flush(&mut pending, &mut out);
    out
}

fn make_track(
    file: &str,
    number: u32,
    title: &str,
    performer: &str,
    album_performer: &str,
    start: f64,
) -> Track {
    let performer = if performer.is_empty() {
        album_performer
    } else {
        performer
    };
    let song = if title.is_empty() {
        format!("Track {number:02}")
    } else {
        title.to_string()
    };
    let label = if performer.is_empty() {
        format!("{number:02} {song}")
    } else {
        format!("{number:02} {performer} - {song}")
    };
    Track {
        location: file.to_string(),
        title: label,
        start,
        end: None,
        is_group: false,
        missing: false,
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    s.split_whitespace().next().unwrap_or("").to_string()
}

fn resolve_file(dir: &Path, name: &str) -> String {
    let p = PathBuf::from(name);
    if p.is_absolute() {
        return p.to_string_lossy().to_string();
    }
    dir.join(p).to_string_lossy().to_string()
}

fn index_seconds(ts: &str) -> Option<f64> {
    let mut p = ts.split(':');
    let m: f64 = p.next()?.parse().ok()?;
    let s: f64 = p.next()?.parse().ok()?;
    let f: f64 = p.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    Some(m * 60.0 + s + f / 75.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_tracks_and_sets_end() {
        let text = r#"
PERFORMER "甲壳虫"
TITLE "专辑"
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    TITLE "一"
    PERFORMER "A"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "二"
    INDEX 01 01:30:00
"#;
        let tracks = parse_cue(text, Path::new("D:/music/x.cue"));
        assert_eq!(tracks.len(), 2);
        assert!(tracks[0].location.replace('/', "\\").ends_with("album.flac"));
        assert!((tracks[0].end.unwrap() - 90.0).abs() < 0.001);
        assert!(tracks[0].title.contains('A'));
        assert!(tracks[1].title.contains('二'));
        assert!(tracks[1].end.is_none());
    }
}
