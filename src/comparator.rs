use crate::scanner::RepoPackages;
use crate::version::{compare_versions, VersionComparison};
use crate::makefile::PackageInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareStatus {
    UpstreamNewer,
    UpstreamOnly,
    MineOnly,
    Same,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Overwrite,
    Add,
    Keep,
    Skip,
}

#[derive(Debug, Clone)]
pub struct CompareResult {
    pub name: String,
    pub mine: Option<PackageInfo>,
    pub upstream: Option<PackageInfo>,
    pub status: CompareStatus,
    pub action: Action,
    pub selected: bool,
}

impl CompareResult {
    pub fn my_version(&self) -> &str {
        self.mine.as_ref().map(|p| p.version.as_str()).unwrap_or("")
    }

    pub fn upstream_version(&self) -> &str {
        self.upstream
            .as_ref()
            .map(|p| p.version.as_str())
            .unwrap_or("")
    }

    pub fn my_path(&self) -> &str {
        self.mine.as_ref().map(|p| p.path.as_str()).unwrap_or("")
    }

    pub fn upstream_path(&self) -> &str {
        self.upstream
            .as_ref()
            .map(|p| p.path.as_str())
            .unwrap_or("")
    }
}

pub fn compare_repos(mine: &RepoPackages, upstream: &RepoPackages) -> Vec<CompareResult> {
    let mut results = Vec::new();
    let mut all_names: Vec<String> = {
        let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
        names.extend(mine.packages.keys().cloned());
        names.extend(upstream.packages.keys().cloned());
        names.into_iter().collect()
    };
    all_names.sort();

    for name in all_names {
        let mine_pkg = mine.packages.get(&name).cloned();
        let upstream_pkg = upstream.packages.get(&name).cloned();

        let (status, action, selected) = match (&mine_pkg, &upstream_pkg) {
            (None, Some(_)) => (CompareStatus::UpstreamOnly, Action::Add, true),
            (Some(_), None) => (CompareStatus::MineOnly, Action::Keep, false),
            (Some(m), Some(u)) => {
                let cmp = compare_versions(&m.version, &u.version);
                match cmp {
                    VersionComparison::UpstreamNewer => {
                        (CompareStatus::UpstreamNewer, Action::Overwrite, true)
                    }
                    VersionComparison::Same => (CompareStatus::Same, Action::Keep, false),
                    VersionComparison::MyNewer => (CompareStatus::Same, Action::Keep, false),
                    VersionComparison::Unknown => (CompareStatus::Manual, Action::Keep, false),
                }
            }
            (None, None) => continue,
        };

        results.push(CompareResult {
            name,
            mine: mine_pkg,
            upstream: upstream_pkg,
            status,
            action,
            selected,
        });
    }

    results
}
