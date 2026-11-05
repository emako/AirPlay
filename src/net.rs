use std::time::Duration;

pub fn fetch(url: &str) -> Result<(Vec<u8>, String), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .build();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("网络请求失败: {e}"))?;
    let ctype = response.content_type().to_string();
    let mut reader = response.into_reader();
    let mut data = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    let limit = 80 * 1024 * 1024;
    loop {
        if data.len() >= limit {
            break;
        }
        let n = std::io::Read::read(&mut reader, &mut chunk).map_err(|e| format!("读取失败: {e}"))?;
        if n == 0 {
            break;
        }
        let room = limit - data.len();
        data.extend_from_slice(&chunk[..n.min(room)]);
    }
    if data.is_empty() {
        return Err("网络音频为空".into());
    }
    Ok((data, guess_ext(url, &ctype)))
}

fn guess_ext(url: &str, ctype: &str) -> String {
    let c = ctype.to_ascii_lowercase();
    if c.contains("mpeg") || c.contains("mp3") {
        return "mp3".into();
    }
    if c.contains("flac") {
        return "flac".into();
    }
    if c.contains("ogg") || c.contains("opus") {
        return "ogg".into();
    }
    if c.contains("wav") {
        return "wav".into();
    }
    if c.contains("mp4") || c.contains("aac") || c.contains("m4a") {
        return "m4a".into();
    }
    let path = url.split('?').next().unwrap_or(url);
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "mp3".into())
}
