#[derive(Clone, Debug)]
pub struct LrcLine {
    pub ms: i64,
    pub text: String,
}

pub fn parse(text: &str) -> Vec<LrcLine> {
    let mut offset: i64 = 0;
    let mut lines = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let mut rest = line;
        let mut stamps = Vec::new();
        while rest.starts_with('[') {
            let Some(end) = rest.find(']') else { break };
            let tag = &rest[1..end];
            rest = rest[end + 1..].trim_start();
            if let Some(ms) = parse_stamp(tag) {
                stamps.push(ms);
            } else if let Some(v) = tag
                .get(..7)
                .filter(|p| p.eq_ignore_ascii_case("offset:"))
                .map(|_| &tag[7..])
                .or_else(|| {
                    tag.strip_prefix("offset:")
                        .or_else(|| tag.strip_prefix("OFFSET:"))
                })
            {
                offset = v.trim().parse().unwrap_or(0);
            } else if tag.to_ascii_lowercase().starts_with("offset:") {
                offset = tag[7..].trim().parse().unwrap_or(0);
            }
            if !rest.starts_with('[') {
                break;
            }
        }
        if stamps.is_empty() {
            continue;
        }
        let text = rest.trim().to_string();
        for ms in stamps {
            lines.push(LrcLine {
                ms: ms + offset,
                text: text.clone(),
            });
        }
    }
    lines.sort_by_key(|l| l.ms);
    lines
}

pub fn current(lines: &[LrcLine], pos_ms: i64) -> Option<(usize, &LrcLine)> {
    let mut found = None;
    for (i, line) in lines.iter().enumerate() {
        if line.ms <= pos_ms {
            found = Some((i, line));
        } else {
            break;
        }
    }
    found
}

fn parse_stamp(tag: &str) -> Option<i64> {
    let tag = tag.trim();
    if !tag.chars().next()?.is_ascii_digit() {
        return None;
    }
    let mut parts = tag.split(':');
    let min: i64 = parts.next()?.parse().ok()?;
    let sec_part = parts.next()?;
    if parts.next().is_some() && !sec_part.contains('.') {
        return None;
    }
    let (sec_s, frac) = if let Some((a, b)) = sec_part.split_once(['.', ':']) {
        (a, b)
    } else {
        (sec_part, "")
    };
    let sec: i64 = sec_s.parse().ok()?;
    let mut ms = min.saturating_mul(60_000) + sec.saturating_mul(1000);
    let digits: String = frac.chars().filter(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let extra = match digits.len() {
            1 => digits.parse::<i64>().unwrap_or(0) * 100,
            2 => digits.parse::<i64>().unwrap_or(0) * 10,
            _ => digits[..3].parse::<i64>().unwrap_or(0),
        };
        ms += extra;
    }
    Some(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_stamp_and_offset() {
        let text = "[offset:100]\n[00:01.00][00:02.50]你好\n[00:03]下一句\n";
        let lines = parse(text);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].ms, 1100);
        assert_eq!(lines[0].text, "你好");
        assert_eq!(lines[1].ms, 2600);
        assert_eq!(lines[2].text, "下一句");
        let (i, line) = current(&lines, 2000).unwrap();
        assert_eq!(i, 0);
        assert_eq!(line.text, "你好");
    }
}
