#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoMode {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub locale: String,
    pub mine_mode: RepoMode,
    pub mine_path: String,
    pub upstream_mode: RepoMode,
    pub upstream_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            locale: "en".to_string(),
            mine_mode: RepoMode::Local,
            mine_path: String::new(),
            upstream_mode: RepoMode::Local,
            upstream_path: String::new(),
        }
    }
}
