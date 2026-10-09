use anyhow::{anyhow, Result};
use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature};
use std::path::Path;

pub enum AuthMethod {
    LocalCredentials,
    Token(String),
}

/// Returns the remote URL of `origin` for the given repo, or an error string.
pub fn get_origin_url(repo_path: &Path) -> String {
    Repository::discover(repo_path)
        .ok()
        .and_then(|repo| {
            repo.find_remote("origin").ok().and_then(|r| {
                r.url().map(|u| u.to_string())
            })
        })
        .unwrap_or_else(|| "(no origin remote)".to_string())
}

/// Returns the actual git workdir root (may differ from the path passed in).
fn repo_root(repo_path: &Path) -> String {
    Repository::discover(repo_path)
        .ok()
        .and_then(|r| r.workdir().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string())
}

pub fn commit_and_push(
    repo_path: &Path,
    commit_message: &str,
    branch: &str,
    auth: &AuthMethod,
    log: &dyn Fn(String),
) -> Result<()> {
    let repo = Repository::discover(repo_path)
        .map_err(|e| anyhow!("Cannot find git repo from '{}': {}", repo_path.display(), e))?;

    let origin_url = get_origin_url(repo_path);
    let root = repo_root(repo_path);
    log(format!("Repository root: {}", root));
    log(format!("Remote origin:   {}", origin_url));
    log(format!("Branch:          {}", branch));

    // Determine which subdirectory to stage (relative to repo workdir)
    let workdir = repo.workdir().ok_or_else(|| anyhow!("bare repository not supported"))?;
    let abs_repo = repo_path.canonicalize().unwrap_or_else(|_| repo_path.to_path_buf());
    let abs_work = workdir.canonicalize().unwrap_or_else(|_| workdir.to_path_buf());
    let rel_path = abs_repo
        .strip_prefix(&abs_work)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let stage_glob = if rel_path == "." || rel_path.is_empty() {
        "*".to_string()
    } else {
        format!("{}/**", rel_path.trim_end_matches('/'))
    };
    log(format!("Staging: {} ...", stage_glob));
    let mut index = repo.index()?;
    index.add_all([stage_glob.as_str()].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    let sig = repo
        .signature()
        .unwrap_or_else(|_| Signature::now("packages_updater", "packages_updater@local").unwrap());

    log(format!("Author: {} <{}>", sig.name().unwrap_or(""), sig.email().unwrap_or("")));

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<git2::Commit> = parent_commit.into_iter().collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    log(format!("Committing: \"{}\"", commit_message));
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        commit_message,
        &tree,
        &parent_refs,
    )?;
    log("Commit created.".to_string());

    log(format!("Pushing to origin/{} ...", branch));
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| anyhow!("remote 'origin' not found in repository at {}", repo_path.display()))?;

    let mut callbacks = RemoteCallbacks::new();
    setup_auth_callbacks(&mut callbacks, auth);

    // Transfer progress callback
    callbacks.transfer_progress(|stats| {
        // just needs to return true to continue
        let _ = stats.received_objects();
        true
    });

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    remote.push(
        &[&format!("refs/heads/{}:refs/heads/{}", branch, branch)],
        Some(&mut push_opts),
    )?;

    log(format!("✓ Pushed to {} ({})", origin_url, branch));
    Ok(())
}

fn setup_auth_callbacks(callbacks: &mut RemoteCallbacks, auth: &AuthMethod) {
    match auth {
        AuthMethod::LocalCredentials => {
            callbacks.credentials(|url, username_from_url, allowed_types| {
                if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                    let username = username_from_url.unwrap_or("git");
                    Cred::ssh_key_from_agent(username)
                } else if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                    if let Ok(cfg) = git2::Config::open_default() {
                        Cred::credential_helper(&cfg, url, username_from_url)
                    } else {
                        Cred::default()
                    }
                } else {
                    Cred::default()
                }
            });
        }
        AuthMethod::Token(token) => {
            let token = token.clone();
            callbacks.credentials(move |_url, _username, _allowed_types| {
                Cred::userpass_plaintext("x-access-token", &token)
            });
        }
    }
}

pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    let repo = Repository::discover(repo_path)
        .map_err(|e| anyhow!("Cannot find git repo from '{}': {}", repo_path.display(), e))?;
    let head = repo.head()?;
    let branch = head.shorthand().unwrap_or("main").to_string();
    Ok(branch)
}
