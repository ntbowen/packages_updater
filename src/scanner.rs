use crate::makefile::{parse_makefile, PackageInfo};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct RepoPackages {
    pub packages: HashMap<String, PackageInfo>,
}

pub fn scan_local_repo(
    repo_path: &Path,
    progress: Option<&dyn Fn(String)>,
) -> Result<RepoPackages> {
    if !repo_path.exists() {
        return Err(anyhow!("Path does not exist: {}", repo_path.display()));
    }
    let mut packages = HashMap::new();

    for entry in WalkDir::new(repo_path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "Makefile" && entry.depth() >= 2 {
            let makefile_path = entry.path();
            match parse_makefile(makefile_path) {
                Ok(info) if !info.name.is_empty() => {
                    if let Some(cb) = progress {
                        cb(format!("  {} {}", info.name, info.version));
                    }
                    let rel = makefile_path
                        .parent()
                        .and_then(|p| p.strip_prefix(repo_path).ok())
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let mut info_with_rel = info;
                    info_with_rel.path = rel;
                    packages.insert(info_with_rel.name.clone(), info_with_rel);
                }
                _ => {}
            }
        }
    }

    Ok(RepoPackages { packages })
}

/// Scan a remote GitHub repo via the REST API.
/// `repo_url` formats accepted:
///   - https://github.com/owner/repo
///   - owner/repo
pub fn scan_remote_repo(
    repo_url: &str,
    progress: Option<&dyn Fn(String)>,
) -> Result<RepoPackages> {
    let (owner, repo) = parse_github_url(repo_url)?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("packages_updater/0.1")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    if let Some(cb) = progress {
        cb(format!("Fetching tree: {}/{}", owner, repo));
    }
    let tree = fetch_tree(&client, &owner, &repo)?;

    let makefile_paths: Vec<String> = tree
        .iter()
        .filter_map(|item| {
            let path = item.get("path")?.as_str()?;
            let itype = item.get("type")?.as_str()?;
            if itype == "blob" && path.ends_with("/Makefile") {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect();

    let total = makefile_paths.len();
    if let Some(cb) = progress {
        cb(format!("Found {} Makefiles, downloading...", total));
    }

    let mut packages: HashMap<String, PackageInfo> = HashMap::new();
    let mut done = 0usize;

    for path in &makefile_paths {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() < 2 {
            continue;
        }

        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/{}/HEAD/{}",
            owner, repo, path
        );

        let content = match client.get(&raw_url).send() {
            Ok(r) if r.status().is_success() => match r.text() {
                Ok(t) => t,
                Err(_) => continue,
            },
            _ => continue,
        };

        let dir = path.trim_end_matches("/Makefile");
        match parse_makefile_content(&content, dir) {
            Ok(info) if !info.name.is_empty() => {
                done += 1;
                if let Some(cb) = progress {
                    cb(format!("[{}/{}] ✓ {}", done, total, info.name));
                }
                packages.insert(info.name.clone(), info);
            }
            _ => {
                done += 1;
            }
        }
    }

    Ok(RepoPackages { packages })
}

fn parse_makefile_content(content: &str, dir_path: &str) -> Result<PackageInfo> {
    let tmp_path = write_temp_makefile(content)?;
    let mut info = parse_makefile(&tmp_path)?;
    let _ = std::fs::remove_file(&tmp_path);
    info.path = dir_path.to_string();
    Ok(info)
}

fn write_temp_makefile(content: &str) -> Result<PathBuf> {
    use std::io::Write;
    let tmp_path = std::env::temp_dir().join(format!(
        "pkg_updater_{}.mk",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    ));
    let mut f = std::fs::File::create(&tmp_path)?;
    f.write_all(content.as_bytes())?;
    Ok(tmp_path)
}


fn fetch_tree(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/HEAD?recursive=1",
        owner, repo
    );
    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "GitHub API error {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        ));
    }
    let json: serde_json::Value = resp.json()?;
    let tree = json
        .get("tree")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("GitHub API: 'tree' field not found"))?;

    Ok(tree
        .iter()
        .filter_map(|v| v.as_object().cloned())
        .collect())
}

fn parse_github_url(url: &str) -> Result<(String, String)> {
    let url = url.trim().trim_end_matches('/');

    if url.starts_with("https://github.com/") {
        let path = url.trim_start_matches("https://github.com/");
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    } else if url.starts_with("http://github.com/") {
        let path = url.trim_start_matches("http://github.com/");
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    } else if url.contains('/') && !url.contains("://") {
        let parts: Vec<&str> = url.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    Err(anyhow!(
        "Cannot parse GitHub URL: '{}'. Expected format: https://github.com/owner/repo",
        url
    ))
}

pub fn package_dir_for(repo_path: &Path, pkg_info: &PackageInfo) -> PathBuf {
    repo_path.join(&pkg_info.path)
}
