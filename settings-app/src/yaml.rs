//! YASB's config.yaml: read it as a tree, change single values in the text (so your comments and layout stay as they are).

use std::{fs, path::PathBuf};
use yaml_rust2::{Yaml, YamlLoader};

pub struct Config {
    pub path: PathBuf,
    pub text: String,
    pub doc: Yaml,
    pub error: Option<String>,
    stamp: Option<std::time::SystemTime>,
    crlf: bool,
    /// edits that are not written to the file yet
    pub dirty: bool,
}

/// The parser is stricter than YASB's: a multi-line `[ ... ]` closed at the indentation of its key is refused. For reading, such lists are
/// joined onto one line (comments inside them dropped). The file itself is never written from this copy.
fn flatten_lists(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let opens = line.split_once(':').map(|(_, rest)| split_comment(rest).0).filter(|v| v.trim_start().starts_with('[') && v.matches('[').count() > v.matches(']').count());
        if let Some(first) = opens {
            let mut joined = format!("{}:{}", line.split_once(':').unwrap().0, if first.is_empty() { String::new() } else { format!(" {}", first.trim_start()) });
            let mut depth = first.matches('[').count() as i32 - first.matches(']').count() as i32;
            let mut j = i + 1;
            while j < lines.len() && depth > 0 {
                let (code, _) = split_comment(lines[j]);
                depth += code.matches('[').count() as i32 - code.matches(']').count() as i32;
                joined.push(' ');
                joined.push_str(code.trim());
                j += 1;
            }
            out.push(joined.replace(", ]", "]").replace(",]", "]"));
            i = j;
        } else {
            out.push(line.to_string());
            i += 1;
        }
    }
    out.join("\n")
}

/// "󰅛" (a UTF-16 pair, as Python writes icons) -> "\U000F055B", which this parser knows.
fn join_surrogates(text: &str) -> String {
    let pair = regex::Regex::new(r"(?i)\\u(d[89ab][0-9a-f]{2})\\u(d[c-f][0-9a-f]{2})").unwrap();
    pair.replace_all(text, |m: &regex::Captures| {
        let (hi, lo) = (u32::from_str_radix(&m[1], 16).unwrap_or(0), u32::from_str_radix(&m[2], 16).unwrap_or(0));
        format!("\\U{:08x}", 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))
    }).into_owned()
}

fn parse(text: &str) -> (Yaml, Option<String>) {
    match YamlLoader::load_from_str(&join_surrogates(&flatten_lists(text))) {
        Ok(mut docs) if !docs.is_empty() => (docs.remove(0), None),
        Ok(_) => (Yaml::BadValue, Some("config.yaml is empty".into())),
        Err(e) => (Yaml::BadValue, Some(format!("config.yaml: {e}"))),
    }
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

/// "key" or '"key"' at the start of a line followed by a colon.
fn key_of(line: &str) -> Option<String> {
    let t = line.trim_start();
    if t.starts_with('#') || t.starts_with('-') {
        return None;
    }
    let (k, _) = t.split_once(':')?;
    Some(k.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
}

/// The text of a value as it is written on its line, without a trailing comment: (value, the comment with its leading spaces).
fn split_comment(rest: &str) -> (String, String) {
    let (mut quote, mut prev) = (None::<char>, ' ');
    for (i, c) in rest.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' && prev.is_whitespace() => return (rest[..i].trim_end().to_string(), format!("  {}", &rest[i..])),
            _ => {}
        }
        prev = c;
    }
    (rest.trim_end().to_string(), String::new())
}

impl Config {
    pub fn load(path: PathBuf) -> Config {
        let raw = fs::read_to_string(&path).unwrap_or_default();
        let crlf = raw.contains("\r\n"); // written back the way it was found
        let text = raw.replace("\r\n", "\n");
        let (doc, error) = parse(&text);
        let stamp = fs::metadata(&path).and_then(|m| m.modified()).ok();
        Config { path, text, doc, error, stamp, crlf, dirty: false }
    }

    /// Read the file again if something else changed it (YASB, you in an editor).
    pub fn reload_if_changed(&mut self) -> bool {
        if self.dirty {
            return false; // your edits are waiting: do not throw them away
        }
        let now = fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if now != self.stamp {
            *self = Config::load(self.path.clone());
            return true;
        }
        false
    }

    /// The node at a path of keys.
    pub fn at(&self, path: &[&str]) -> &Yaml {
        let mut node = &self.doc;
        for k in path {
            node = &node[*k];
        }
        node
    }

    pub fn bool(&self, path: &[&str], default: bool) -> bool {
        match self.at(path) {
            Yaml::Boolean(b) => *b,
            Yaml::String(s) if s.eq_ignore_ascii_case("true") => true,
            Yaml::String(s) if s.eq_ignore_ascii_case("false") => false,
            _ => default,
        }
    }

    /// The `i`th entry of a list.
    pub fn item(&self, path: &[&str], i: usize) -> &Yaml {
        &self.at(path)[i]
    }

    pub fn item_len(&self, path: &[&str]) -> usize {
        self.at(path).as_vec().map(|v| v.len()).unwrap_or(0)
    }

    /// A value of the `i`th list entry as text ("" when it is not there).
    pub fn item_string(&self, path: &[&str], i: usize, field: &str) -> String {
        match &self.item(path, i)[field] {
            Yaml::String(s) => s.clone(),
            Yaml::Integer(n) => n.to_string(),
            Yaml::Real(r) => r.clone(),
            Yaml::Boolean(b) => if *b { "True".into() } else { "False".into() },
            _ => String::new(),
        }
    }

    pub fn int(&self, path: &[&str], default: i64) -> i64 {
        self.at(path).as_i64().unwrap_or(default)
    }

    pub fn string(&self, path: &[&str]) -> String {
        match self.at(path) {
            Yaml::String(s) => s.clone(),
            Yaml::Integer(i) => i.to_string(),
            Yaml::Real(r) => r.clone(),
            Yaml::Boolean(b) => b.to_string(),
            _ => String::new(),
        }
    }

    pub fn list(&self, path: &[&str]) -> Vec<String> {
        self.at(path).as_vec().map(|v| v.iter().map(|x| match x {
            Yaml::String(s) => s.clone(),
            Yaml::Integer(i) => i.to_string(),
            other => format!("{other:?}"),
        }).collect()).unwrap_or_default()
    }

    /// The keys of a mapping, in file order.
    pub fn keys(&self, path: &[&str]) -> Vec<String> {
        self.at(path).as_hash().map(|h| h.keys().filter_map(|k| k.as_str().map(String::from)).collect()).unwrap_or_default()
    }

    // --- writing -----------------------------------------------------------------------------------------------------------------------

    fn lines(&self) -> Vec<String> {
        self.text.split('\n').map(String::from).collect()
    }

    /// The line of the key at `path`, and the first line after its block.
    fn find(&self, lines: &[String], path: &[&str]) -> Option<(usize, usize)> {
        let (mut start, mut end, mut min_indent) = (0usize, lines.len(), 0usize);
        let mut found = None;
        for (depth, seg) in path.iter().enumerate() {
            let mut hit = None;
            let mut block_indent = None;
            for i in start..end {
                if is_blank_or_comment(&lines[i]) {
                    continue;
                }
                let ind = indent_of(&lines[i]);
                if depth > 0 && ind < min_indent {
                    break;
                }
                let block = *block_indent.get_or_insert(ind);
                if ind == block && key_of(&lines[i]).as_deref() == Some(seg) {
                    hit = Some(i);
                    break;
                }
            }
            let i = hit?;
            let ind = indent_of(&lines[i]);
            let mut stop = end;
            for j in i + 1..end {
                if !is_blank_or_comment(&lines[j]) && indent_of(&lines[j]) <= ind {
                    stop = j;
                    break;
                }
            }
            found = Some((i, stop));
            (start, end, min_indent) = (i + 1, stop, ind + 1);
        }
        found
    }

    /// Write the waiting edits to the file.
    pub fn apply(&mut self) -> Result<(), String> {
        let on_disk = if self.crlf { self.text.replace('\n', "\r\n") } else { self.text.clone() };
        let result = fs::write(&self.path, on_disk).map_err(|e| e.to_string());
        self.stamp = fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if result.is_ok() {
            self.dirty = false;
        }
        result
    }

    /// Throw the waiting edits away: back to what is in the file.
    pub fn discard(&mut self) {
        *self = Config::load(self.path.clone());
    }

    fn save(&mut self, lines: Vec<String>) -> Result<(), String> {
        self.text = lines.join("\n");
        let (doc, error) = parse(&self.text);
        if let Some(e) = error {
            return Err(e);
        }
        self.doc = doc;
        self.dirty = true; // written to the file by apply()
        Ok(())
    }

    /// Add the missing keys of `path` (with `value` on the last) under the part of it that exists.
    fn insert(&self, lines: &mut Vec<String>, path: &[&str], value: &str) {
        for cut in (0..path.len()).rev() {
            let parent = &path[..cut];
            let at = if parent.is_empty() { Some((usize::MAX, lines.len())) } else { self.find(lines, parent) };
            let Some((pi, stop)) = at else { continue };
            let child_indent = if parent.is_empty() { 0 } else {
                (pi + 1..stop).find(|&j| !is_blank_or_comment(&lines[j])).map(|j| indent_of(&lines[j])).unwrap_or(indent_of(&lines[pi]) + 2)
            };
            let mut new: Vec<String> = Vec::new();
            for (n, key) in path[cut..].iter().enumerate() {
                let pad = " ".repeat(child_indent + n * 2);
                if cut + n + 1 == path.len() { new.push(format!("{pad}{key}: {value}")) } else { new.push(format!("{pad}{key}:")) }
            }
            let at_line = if parent.is_empty() { lines.len() } else { pi + 1 };
            for (n, l) in new.into_iter().enumerate() {
                lines.insert(at_line + n, l);
            }
            return;
        }
    }

    /// Set a single value (`value` is already written the way YAML wants it: 12, true, "text").
    pub fn set(&mut self, path: &[&str], value: &str) -> Result<(), String> {
        let mut lines = self.lines();
        match self.find(&lines, path) {
            Some((i, _)) => {
                let line = lines[i].clone();
                let colon = line.find(':').ok_or("not a key line")?;
                let (_, comment) = split_comment(&line[colon + 1..]);
                lines[i] = format!("{}: {value}{comment}", &line[..colon]);
            }
            None => self.insert(&mut lines, path, value),
        }
        self.save(lines)
    }

    /// Replace a key's whole value, however many lines it takes now (a gradient is a block), with one inline value.
    pub fn set_inline(&mut self, path: &[&str], value: &str) -> Result<(), String> {
        let mut lines = self.lines();
        let (i, stop) = self.find(&lines, path).ok_or("that key is not in the file")?;
        let indent = indent_of(&lines[i]);
        if let Some(last) = (i + 1..stop).rev().find(|&j| !is_blank_or_comment(&lines[j]) && indent_of(&lines[j]) > indent) {
            lines.drain(i + 1..=last);
        }
        let line = lines[i].clone();
        let colon = line.find(':').ok_or("not a key line")?;
        let (_, comment) = split_comment(&line[colon + 1..]);
        lines[i] = format!("{}: {value}{comment}", &line[..colon]);
        self.save(lines)
    }

    pub fn set_bool(&mut self, path: &[&str], v: bool) -> Result<(), String> {
        self.set(path, if v { "true" } else { "false" })
    }

    pub fn set_int(&mut self, path: &[&str], v: i64) -> Result<(), String> {
        self.set(path, &v.to_string())
    }

    /// Text: quoted the way the file has it (double quotes when it was plain).
    pub fn set_text(&mut self, path: &[&str], v: &str) -> Result<(), String> {
        let looks_plain = matches!(v, "auto" | "None" | "none");
        self.set(path, &if looks_plain { v.to_string() } else { format!("\"{}\"", v.replace('"', "\\\"")) })
    }

    /// For a block list under `path`: where each entry starts and where its last real line is.
    fn item_ranges(&self, lines: &[String], path: &[&str]) -> Option<(usize, Vec<(usize, usize)>)> {
        let (key_line, stop) = self.find(lines, path)?;
        let starts: Vec<usize> = (key_line + 1..stop).filter(|&j| lines[j].trim_start().starts_with("- ") && indent_of(&lines[j]) > indent_of(&lines[key_line])).collect();
        let first_indent = starts.first().map(|&j| indent_of(&lines[j]))?;
        let starts: Vec<usize> = starts.into_iter().filter(|&j| indent_of(&lines[j]) == first_indent).collect();
        let ranges = starts.iter().enumerate().map(|(n, &st)| {
            let next = starts.get(n + 1).copied().unwrap_or(stop);
            let end = (st..next).rev().find(|&j| !is_blank_or_comment(&lines[j])).unwrap_or(st);
            (st, end)
        }).collect();
        Some((key_line, ranges))
    }

    /// Change one value of the `idx`th entry of a block list (added to the entry when it is not there yet).
    pub fn set_item_field(&mut self, path: &[&str], idx: usize, field: &str, value: &str) -> Result<(), String> {
        let mut lines = self.lines();
        let (_, ranges) = self.item_ranges(&lines, path).ok_or("that list is not in the file")?;
        let (start, end) = *ranges.get(idx).ok_or("no such entry")?;
        for j in start..=end {
            let content = lines[j].trim_start().trim_start_matches("- ").trim_start();
            if let Some(rest) = content.strip_prefix(field).and_then(|r| r.strip_prefix(':')) {
                let (_, comment) = split_comment(rest);
                let head = &lines[j][..lines[j].len() - rest.len()];
                lines[j] = format!("{head} {value}{comment}");
                return self.save(lines);
            }
        }
        let pad = " ".repeat(indent_of(&lines[start]) + 2);
        lines.insert(end + 1, format!("{pad}{field}: {value}"));
        self.save(lines)
    }

    /// Take the `idx`th entry out of a block list.
    pub fn remove_item(&mut self, path: &[&str], idx: usize) -> Result<(), String> {
        let mut lines = self.lines();
        let (_, ranges) = self.item_ranges(&lines, path).ok_or("that list is not in the file")?;
        let (start, end) = *ranges.get(idx).ok_or("no such entry")?;
        lines.drain(start..=end);
        self.save(lines)
    }

    /// Add an entry (`field: value` lines) at the end of a block list.
    pub fn append_item(&mut self, path: &[&str], fields: &[(&str, &str)]) -> Result<(), String> {
        let mut lines = self.lines();
        let (key_line, ranges) = match self.item_ranges(&lines, path) {
            Some(r) => r,
            None => {
                let (k, _) = self.find(&lines, path).ok_or("that list is not in the file")?;
                (k, Vec::new())
            }
        };
        let pad = " ".repeat(match ranges.first() { Some(&(st, _)) => indent_of(&lines[st]), None => indent_of(&lines[key_line]) + 2 });
        let at = ranges.last().map(|&(_, e)| e + 1).unwrap_or(key_line + 1);
        let mut new: Vec<String> = Vec::new();
        for (i, (k, v)) in fields.iter().enumerate() {
            new.push(if i == 0 { format!("{pad}- {k}: {v}") } else { format!("{pad}  {k}: {v}") });
        }
        if !ranges.is_empty() {
            new.insert(0, String::new()); // a blank line between entries, like the file has
        }
        for (n, l) in new.into_iter().enumerate() {
            lines.insert(at + n, l);
        }
        self.save(lines)
    }

    /// A list of strings, written as a flow list. Commented-out items inside the old list are kept.
    pub fn set_list(&mut self, path: &[&str], items: &[String]) -> Result<(), String> {
        let mut lines = self.lines();
        let Some((i, _)) = self.find(&lines, path) else {
            let flow = format!("[{}]", items.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", "));
            self.insert(&mut lines, path, &flow);
            return self.save(lines);
        };
        let ind = indent_of(&lines[i]);
        let colon = lines[i].find(':').ok_or("not a key line")?;
        // the list may span several lines: up to the line where its brackets balance
        let (mut depth, mut last) = (0i32, i);
        'scan: for j in i..lines.len() {
            let body = if j == i { lines[j][colon + 1..].to_string() } else { lines[j].clone() };
            let (code, _) = split_comment(&body);
            for c in code.chars() {
                depth += match c { '[' => 1, ']' => -1, _ => 0 };
            }
            last = j;
            if depth <= 0 && (j > i || code.contains('[')) {
                break 'scan;
            }
        }
        let kept: Vec<String> = (i + 1..=last).filter(|&j| lines[j].trim_start().starts_with('#')).map(|j| lines[j].clone()).collect();
        let key = lines[i][..colon].to_string();
        let pad = " ".repeat(ind + 2);
        let mut new = Vec::new();
        if items.len() <= 3 && kept.is_empty() {
            new.push(format!("{key}: [{}]", items.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ")));
        } else {
            new.push(format!("{key}: ["));
            new.extend(items.iter().map(|s| format!("{pad}\"{s}\",")));
            new.extend(kept);
            new.push(format!("{}]", " ".repeat(ind)));
        }
        lines.splice(i..=last, new);
        self.save(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_list_entries_keeps_the_rest() {
        let original = fs::read_to_string(std::env::var("USERPROFILE").unwrap() + "/.config/tacky-borders/config.yaml").unwrap();
        let path = std::env::temp_dir().join("tacky-test.yaml");
        fs::write(&path, &original).unwrap();
        let mut c = Config::load(path.clone());
        assert!(c.error.is_none(), "{:?}", c.error);
        let rules = c.item_len(&["window_rules"]);
        c.set_item_field(&["global", "animations", "active"], 0, "duration", "500").unwrap();
        c.set_item_field(&["window_rules"], 1, "enabled", "True").unwrap();
        c.append_item(&["window_rules"], &[("match", "Process"), ("name", "\"Spotify\""), ("enabled", "False")]).unwrap();
        c.remove_item(&["window_rules"], 0).unwrap();
        c.set_bool(&["global", "follow_native_border"], false).unwrap();
        c.apply().unwrap();
        let after = Config::load(path);
        assert_eq!(after.item_string(&["global", "animations", "active"], 0, "duration"), "500");
        assert_eq!(after.item_len(&["window_rules"]), rules); // one added, one removed
        assert_eq!(after.item_string(&["window_rules"], rules - 1, "name"), "Spotify");
        assert_eq!(after.item_string(&["window_rules"], 0, "enabled").to_lowercase(), "true"); // what was entry 1 is now entry 0
        assert!(!after.bool(&["global", "follow_native_border"], true));
        // everything else in the file is untouched: the line count differs only by the added and removed entries
        let (a, b) = (original.lines().count() as i64, fs::read_to_string(after.path.clone()).unwrap().lines().count() as i64);
        println!("lines {a} -> {b}");
    }
}
