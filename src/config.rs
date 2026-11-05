use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Ini {
    sections: BTreeMap<String, Vec<(String, String)>>,
}

impl Ini {
    pub fn load(path: &Path) -> Self {
        let Ok(text) = fs::read(path) else {
            return Self::default();
        };
        let text = String::from_utf8(text)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
        let text = text.trim_start_matches('\u{feff}');
        let mut ini = Self::default();
        let mut section = "main".to_string();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
                section = line[1..line.len() - 1].trim().to_ascii_lowercase();
                ini.sections.entry(section.clone()).or_default();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            ini.sections
                .entry(section.clone())
                .or_default()
                .push((k.trim().to_string(), v.trim().to_string()));
        }
        ini
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        for (sec, keys) in &self.sections {
            out.push('[');
            out.push_str(sec);
            out.push_str("]\r\n");
            for (k, v) in keys {
                out.push_str(k);
                out.push('=');
                out.push_str(v);
                out.push_str("\r\n");
            }
            out.push_str("\r\n");
        }
        fs::write(path, out)
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let sec = self.sections.get(&section.to_ascii_lowercase())?;
        sec.iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let sec = self
            .sections
            .entry(section.to_ascii_lowercase())
            .or_default();
        if let Some((_, v)) = sec.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
            *v = value.to_string();
        } else {
            sec.push((key.to_string(), value.to_string()));
        }
    }

    pub fn get_bool(&self, section: &str, key: &str, default: bool) -> bool {
        match self.get(section, key).as_deref().map(str::trim) {
            Some("1") | Some("true") | Some("True") | Some("yes") | Some("YES") | Some("on") => {
                true
            }
            Some("0") | Some("false") | Some("False") | Some("no") | Some("NO") | Some("off") => {
                false
            }
            Some(s) => parse_bool_loose(s).unwrap_or(default),
            None => default,
        }
    }

    pub fn get_i32(&self, section: &str, key: &str, default: i32) -> i32 {
        self.get(section, key)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(default)
    }

    pub fn set_bool(&mut self, section: &str, key: &str, value: bool) {
        self.set(section, key, if value { "1" } else { "0" });
    }

    pub fn set_i32(&mut self, section: &str, key: &str, value: i32) {
        self.set(section, key, &value.to_string());
    }
}

fn parse_bool_loose(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn roundtrip_keeps_unknown_keys() {
        let mut ini = Ini::default();
        ini.set("main", "custom_name", "AirPlay");
        ini.set("main", "vendor_extra", "keep");
        ini.set_bool("main", "random", true);
        ini.set_i32("main", "last_song", 3);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("airplay-ini-{nanos}.ini"));
        ini.save(&path).unwrap();
        let loaded = Ini::load(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded.get("main", "custom_name").as_deref(), Some("AirPlay"));
        assert_eq!(loaded.get("main", "vendor_extra").as_deref(), Some("keep"));
        assert!(loaded.get_bool("main", "random", false));
        assert_eq!(loaded.get_i32("main", "last_song", -1), 3);
        assert!(loaded.get_bool("main", "missing", true));
    }
}
