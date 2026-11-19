//! Icons in labels are typed the way YASB's config.yaml has them: as UTF-16 escapes (`\udb81\ude33`, `\uf47c`).

/// Text with everything outside plain ASCII written as `\uXXXX` (two of them for a symbol above U+FFFF).
pub fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for u in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{:04x}", u));
            }
        }
    }
    out
}

/// The reverse: `\uXXXX` (surrogate pairs are joined) and `U+XXXXX` become the real characters; everything else stays.
pub fn unescape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut units: Vec<u16> = Vec::new();
    let mut out = String::new();
    let flush = |units: &mut Vec<u16>, out: &mut String| {
        out.extend(char::decode_utf16(units.drain(..)).map(|r| r.unwrap_or('\u{fffd}')));
    };
    let hex = |from: usize, len: usize| -> Option<u32> {
        let h: String = chars.get(from..from + len)?.iter().collect();
        u32::from_str_radix(&h, 16).ok()
    };
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1) == Some(&'u') {
            if let Some(v) = hex(i + 2, 4) {
                units.push(v as u16);
                i += 6;
                continue;
            }
        }
        if chars[i] == 'U' && chars.get(i + 1) == Some(&'+') {
            let n = (i + 2..chars.len()).take_while(|&j| chars[j].is_ascii_hexdigit()).count().min(6);
            if let Some(v) = hex(i + 2, n).filter(|_| n >= 4) {
                if let Some(c) = char::from_u32(v) {
                    flush(&mut units, &mut out);
                    out.push(c);
                    i += 2 + n;
                    continue;
                }
            }
        }
        flush(&mut units, &mut out);
        out.push(chars[i]);
        i += 1;
    }
    flush(&mut units, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        assert_eq!(escape("\u{f0c33}x"), "\\udb83\\udc33x");
        assert_eq!(unescape("\\udb83\\udc33x"), "\u{f0c33}x");
        assert_eq!(unescape("U+F47C"), "\u{f47c}");
        assert_eq!(unescape("{icon} hi"), "{icon} hi");
    }
}
