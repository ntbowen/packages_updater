# packages_updater

An interactive TUI tool for syncing your OpenWrt packages fork with upstream.

## Features

- Full-screen TUI (ratatui) with step-by-step workflow
- i18n support: English / 中文
- Smart version comparison: semver, date (YYYY-MM-DD), numeric
- Auto-overwrite when upstream is newer; auto-add upstream-only packages
- Manual review mode for ambiguous version differences
- Export comparison report to `.xlsx` or `.csv`
- Git commit + push with SSH key or GitHub PAT authentication

## Workflow

```
1. Language Select
2. My Repository  (local path or GitHub URL)
3. Upstream Repo  (local path or GitHub URL)
4. Scan & Compare (progress bar)
5. Results Table  (tabbed: upstream_newer / upstream_only / mine_only / same / manual)
6. Export Report  (xlsx / csv / skip)
7. Apply Changes  (copy package dirs from upstream → mine)
8. Git Push       (local credentials or PAT)
```

## Key Bindings

| Key | Action |
|-----|--------|
| `↑ ↓` | Navigate list |
| `Tab` | Switch field / tab |
| `Enter` | Confirm |
| `Space` | Toggle selection (manual review tab) |
| `a` | Select all (manual review tab) |
| `d` | Deselect all (manual review tab) |
| `Ctrl+C` | Quit |

## Build

```bash
cargo build --release
./target/release/packages_updater
```

## Notes

- The tool only copies package directories (Makefile + patches + files), it does not modify git history until the push step.
- "Mine only" packages (present in your fork but deleted upstream) are **always kept** by default.
- Remote GitHub URL mode uses the GitHub REST API v3 (unauthenticated: 60 req/hr limit). For large repos (500+ packages), use local mode.
