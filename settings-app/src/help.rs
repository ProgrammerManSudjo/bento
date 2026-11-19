//! What each widget option does, from YASB's own documentation (assets/option_help.tsv, made by tools/Fetch-OptionHelp.ps1).

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Help {
    pub kind: String,
    pub default: String,
    pub text: String,
}

type Table = HashMap<(String, String), Help>;

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = Table::new();
        for line in include_str!("../assets/option_help.tsv").lines() {
            let f: Vec<&str> = line.splitn(5, '\t').collect();
            if f.len() == 5 {
                // the first mention of an option in a doc wins (later tables repeat names for nested settings)
                map.entry((f[0].to_string(), f[1].to_string())).or_insert(Help { kind: f[2].to_string(), default: f[3].to_string(), text: f[4].to_string() });
            }
        }
        map
    })
}

/// The doc a widget type is described in: "yasb.cava.CavaWidget" is "cava", "komorebi.active_layout.ActiveLayoutWidget" is "komorebi_layout".
fn doc_key(widget_type: &str) -> String {
    let parts: Vec<&str> = widget_type.split('.').collect();
    match parts.as_slice() {
        ["komorebi", "active_layout", ..] => "komorebi_layout".into(),
        ["komorebi", module, ..] => format!("komorebi_{module}"),
        ["yasb", "active_window", ..] => "active_windows_title".into(),
        ["yasb", module, ..] => (*module).to_string(),
        [other, ..] => (*other).to_string(),
        [] => String::new(),
    }
}

pub fn lookup(widget_type: &str, option: &str) -> Option<&'static Help> {
    table().get(&(doc_key(widget_type), option.to_string()))
}

impl Help {
    /// One or two lines for under the option's name: what it does, and its default.
    pub fn hint(&self) -> String {
        let mut text: String = self.text.chars().take(230).collect();
        if self.text.chars().count() > 230 {
            text.push_str("...");
        }
        let d = self.default.trim();
        if !d.is_empty() && d != "None" && d != "[]" && d != "\"\"" && !text.to_lowercase().contains("default") {
            text.push_str(&format!("  Default: {}", d.trim_matches('"')));
        }
        text
    }

    /// The fixed set of words the option accepts, when its description lists them ("Can be 'bars', 'waves'...").
    pub fn choices(&self) -> Vec<String> {
        let lower = self.text.to_lowercase();
        if !self.kind.to_lowercase().contains("string") || !["can be", "options", "one of", "possible", "either", "valid", "accepts", "values"].iter().any(|w| lower.contains(w)) {
            return Vec::new();
        }
        static QUOTED: OnceLock<Regex> = OnceLock::new();
        let re = QUOTED.get_or_init(|| Regex::new(r#"['"]([A-Za-z0-9_\-]{1,24})['"]"#).unwrap());
        let mut out: Vec<String> = Vec::new();
        for c in re.captures_iter(&self.text) {
            let w = c[1].to_string();
            if !out.contains(&w) {
                out.push(w);
            }
        }
        if (2..=12).contains(&out.len()) { out } else { Vec::new() }
    }

    /// A number option that is really an on/off switch (0 or 1).
    pub fn is_flag(&self) -> bool {
        let lower = self.text.to_lowercase();
        self.kind.to_lowercase().contains("int") && (lower.contains("1 = on") || lower.contains("1 = enable") || lower.contains("1 to enable"))
    }
}
