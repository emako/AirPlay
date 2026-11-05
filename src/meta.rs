use lofty::prelude::*;
use lofty::probe::Probe;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Meta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub lyrics: String,
    pub cover: Option<Vec<u8>>,
}

pub fn read_meta(path: &Path) -> Meta {
    let mut meta = Meta::default();
    if let Ok(tagged) = Probe::open(path).and_then(|p| p.read()) {
        if let Some(tag) = tagged.primary_tag() {
            fill_tag(&mut meta, tag);
        }
        if meta.lyrics.is_empty() || meta.cover.is_none() {
            for tag in tagged.tags() {
                if meta.title.is_empty() && meta.artist.is_empty() && meta.cover.is_none() {
                    fill_tag(&mut meta, tag);
                } else {
                    if meta.lyrics.is_empty() {
                        if let Some(s) = tag.get_string(&ItemKey::Lyrics) {
                            meta.lyrics = s.to_string();
                        }
                    }
                    if meta.cover.is_none() {
                        if let Some(pic) = tag.pictures().first() {
                            meta.cover = Some(pic.data().to_vec());
                        }
                    }
                }
            }
        }
    }
    if meta.cover.is_none() {
        meta.cover = sibling_cover(path);
    }
    meta
}

fn fill_tag(meta: &mut Meta, tag: &lofty::tag::Tag) {
    if meta.title.is_empty() {
        if let Some(v) = tag.title() {
            meta.title = v.trim().to_string();
        }
    }
    if meta.artist.is_empty() {
        if let Some(v) = tag.artist() {
            meta.artist = v.trim().to_string();
        }
    }
    if meta.album.is_empty() {
        if let Some(v) = tag.album() {
            meta.album = v.trim().to_string();
        }
    }
    if meta.lyrics.is_empty() {
        if let Some(s) = tag.get_string(&ItemKey::Lyrics) {
            meta.lyrics = s.to_string();
        }
    }
    if meta.cover.is_none() {
        if let Some(pic) = tag.pictures().first() {
            meta.cover = Some(pic.data().to_vec());
        }
    }
}

fn sibling_cover(path: &Path) -> Option<Vec<u8>> {
    let dir = path.parent()?;
    for name in [
        "cover.jpg",
        "cover.jpeg",
        "cover.png",
        "folder.jpg",
        "folder.jpeg",
        "folder.png",
        "front.jpg",
        "Front.jpg",
    ] {
        let p = dir.join(name);
        if let Ok(bytes) = std::fs::read(p) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }
    None
}

pub fn display_now(meta: &Meta, fallback: &str) -> String {
    match (meta.artist.is_empty(), meta.title.is_empty()) {
        (false, false) => format!("{} - {}", meta.artist, meta.title),
        (true, false) => meta.title.clone(),
        (false, true) => format!("{} - {fallback}", meta.artist),
        (true, true) => fallback.to_string(),
    }
}
