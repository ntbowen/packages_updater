use crate::comparator::{Action, CompareResult};
use anyhow::{anyhow, Result};
use std::path::Path;

pub fn apply_changes(
    results: &[CompareResult],
    mine_repo: &Path,
    upstream_repo: &Path,
) -> Result<usize> {
    let mut count = 0;

    for result in results {
        match &result.action {
            Action::Overwrite => {
                if let Some(upstream) = &result.upstream {
                    let src_dir = upstream_repo.join(&upstream.path);
                    let dst_dir = if let Some(mine) = &result.mine {
                        mine_repo.join(&mine.path)
                    } else {
                        mine_repo.join(&upstream.path)
                    };
                    copy_package_dir(&src_dir, &dst_dir)?;
                    count += 1;
                }
            }
            Action::Add => {
                if let Some(upstream) = &result.upstream {
                    let src_dir = upstream_repo.join(&upstream.path);
                    let dst_dir = mine_repo.join(&upstream.path);
                    copy_package_dir(&src_dir, &dst_dir)?;
                    count += 1;
                }
            }
            Action::Keep | Action::Skip => {}
        }
    }

    Ok(count)
}

fn copy_package_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Err(anyhow!("source directory not found: {}", src.display()));
    }
    std::fs::create_dir_all(dst)?;
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
