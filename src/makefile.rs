use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub release: String,
    pub source_date: String,
    pub path: String,
}

pub fn parse_makefile(makefile_path: &Path) -> Result<PackageInfo> {
    let content = std::fs::read_to_string(makefile_path)?;
    let variables = extract_variables(&content);

    let mut info = PackageInfo::default();
    info.path = makefile_path
        .parent()
        .unwrap_or(makefile_path)
        .to_string_lossy()
        .to_string();

    info.name = resolve_value(
        variables.get("PKG_NAME").map(|s| s.as_str()).unwrap_or(""),
        &variables,
    );
    info.version = resolve_value(
        variables
            .get("PKG_VERSION")
            .map(|s| s.as_str())
            .unwrap_or(""),
        &variables,
    );
    info.release = resolve_value(
        variables
            .get("PKG_RELEASE")
            .map(|s| s.as_str())
            .unwrap_or(""),
        &variables,
    );
    info.source_date = resolve_value(
        variables
            .get("PKG_SOURCE_DATE")
            .map(|s| s.as_str())
            .unwrap_or(""),
        &variables,
    );

    if info.name.is_empty() {
        if let Some(parent) = makefile_path.parent() {
            if let Some(dir_name) = parent.file_name() {
                info.name = dir_name.to_string_lossy().to_string();
            }
        }
    }

    Ok(info)
}

fn extract_variables(content: &str) -> HashMap<String, String> {
    let mut vars: HashMap<String, String> = HashMap::new();
    let assign_re = Regex::new(r"(?m)^([A-Z_][A-Z0-9_]*)\s*[:?]?=\s*(.*)$").unwrap();

    for cap in assign_re.captures_iter(content) {
        let key = cap[1].trim().to_string();
        let val = cap[2].trim().to_string();
        vars.entry(key).or_insert(val);
    }
    vars
}

fn resolve_value(val: &str, vars: &HashMap<String, String>) -> String {
    let var_ref = Regex::new(r"\$\(([A-Z_][A-Z0-9_]*)\)").unwrap();
    let mut result = val.to_string();
    let mut iterations = 0;
    loop {
        iterations += 1;
        if iterations > 10 {
            break;
        }
        let new_result = var_ref
            .replace_all(&result, |caps: &regex::Captures| {
                vars.get(&caps[1])
                    .map(|s| s.clone())
                    .unwrap_or_else(|| caps[0].to_string())
            })
            .to_string();
        if new_result == result {
            break;
        }
        result = new_result;
    }
    result
}
