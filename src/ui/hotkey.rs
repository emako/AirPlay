use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, VK_DOWN,
    VK_LEFT, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_RIGHT,
    VK_SPACE, VK_UP,
};

pub fn apply(hwnd: HWND, keys: &[String; 12], global: bool) {
    unsafe {
        for id in 1..=16 {
            let _ = UnregisterHotKey(hwnd, id);
        }
        if !global || hwnd.0.is_null() {
            return;
        }
        for (i, spec) in keys.iter().enumerate() {
            if let Some((mods, vk)) = parse(spec) {
                let _ = RegisterHotKey(hwnd, (i as i32) + 1, mods, vk);
            }
        }
        let _ = RegisterHotKey(hwnd, 13, HOT_KEY_MODIFIERS(0), VK_MEDIA_PLAY_PAUSE.0 as u32);
        let _ = RegisterHotKey(hwnd, 14, HOT_KEY_MODIFIERS(0), VK_MEDIA_NEXT_TRACK.0 as u32);
        let _ = RegisterHotKey(hwnd, 15, HOT_KEY_MODIFIERS(0), VK_MEDIA_PREV_TRACK.0 as u32);
        let _ = RegisterHotKey(hwnd, 16, HOT_KEY_MODIFIERS(0), VK_MEDIA_STOP.0 as u32);
    }
}

pub fn parse(spec: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let mut mods = 0u32;
    let mut key = None;
    for part in spec.split('+') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "ctl" => mods |= MOD_CONTROL.0,
            "alt" => mods |= MOD_ALT.0,
            "shift" => mods |= MOD_SHIFT.0,
            other => key = vk(other),
        }
    }
    Some((HOT_KEY_MODIFIERS(mods), key?))
}

fn vk(name: &str) -> Option<u32> {
    let n = name.to_ascii_lowercase();
    let v = match n.as_str() {
        "space" => VK_SPACE.0,
        "left" => VK_LEFT.0,
        "right" => VK_RIGHT.0,
        "up" => VK_UP.0,
        "down" => VK_DOWN.0,
        "next" | "medianext" => VK_MEDIA_NEXT_TRACK.0,
        "prev" | "mediaprev" => VK_MEDIA_PREV_TRACK.0,
        "play" | "mediaplay" => VK_MEDIA_PLAY_PAUSE.0,
        "stop" | "mediastop" => VK_MEDIA_STOP.0,
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap().to_ascii_uppercase();
            if c.is_ascii_alphanumeric() {
                c as u16
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(v as u32)
}
