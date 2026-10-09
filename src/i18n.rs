use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::RwLock;

static LOCALE: RwLock<String> = RwLock::new(String::new());
static TRANSLATIONS: OnceCell<HashMap<String, HashMap<String, String>>> = OnceCell::new();

pub fn init_translations() {
    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();

    let en_toml = include_str!("../locales/en.toml");
    map.insert("en".to_string(), parse_toml(en_toml));

    let zh_toml = include_str!("../locales/zh-CN.toml");
    map.insert("zh-CN".to_string(), parse_toml(zh_toml));

    TRANSLATIONS.set(map).ok();
}

fn parse_toml(content: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let val_raw = line[eq_pos + 1..].trim();
            let val = val_raw
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(val_raw)
                .to_string();
            result.insert(key, val);
        }
    }
    result
}

pub fn set_locale(locale: &str) {
    if let Ok(mut l) = LOCALE.write() {
        *l = locale.to_string();
    }
}

pub fn get_locale() -> String {
    LOCALE
        .read()
        .ok()
        .map(|l| if l.is_empty() { "en".to_string() } else { l.clone() })
        .unwrap_or_else(|| "en".to_string())
}

pub fn t(key: &str) -> String {
    let locale = get_locale();
    let translations = TRANSLATIONS.get().expect("translations not initialized");

    translations
        .get(&locale)
        .and_then(|m| m.get(key))
        .or_else(|| translations.get("en").and_then(|m| m.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

pub fn tf(key: &str, vars: &[(&str, &str)]) -> String {
    let mut result = t(key);
    for (k, v) in vars {
        result = result.replace(&format!("%{{{}}}", k), v);
    }
    result
}

pub fn available_locales() -> Vec<(&'static str, &'static str)> {
    vec![("en", "English"), ("zh-CN", "中文")]
}
