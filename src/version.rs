use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionComparison {
    MyNewer,
    UpstreamNewer,
    Same,
    Unknown,
}

pub fn compare_versions(mine: &str, upstream: &str) -> VersionComparison {
    if mine.is_empty() && upstream.is_empty() {
        return VersionComparison::Same;
    }
    if mine.is_empty() {
        return VersionComparison::UpstreamNewer;
    }
    if upstream.is_empty() {
        return VersionComparison::MyNewer;
    }
    if mine == upstream {
        return VersionComparison::Same;
    }

    if let (Some(a), Some(b)) = (parse_semver(mine), parse_semver(upstream)) {
        return match a.cmp(&b) {
            Ordering::Less => VersionComparison::UpstreamNewer,
            Ordering::Greater => VersionComparison::MyNewer,
            Ordering::Equal => VersionComparison::Same,
        };
    }

    if let (Some(a), Some(b)) = (parse_date(mine), parse_date(upstream)) {
        return match a.cmp(&b) {
            Ordering::Less => VersionComparison::UpstreamNewer,
            Ordering::Greater => VersionComparison::MyNewer,
            Ordering::Equal => VersionComparison::Same,
        };
    }

    if let (Some(a), Some(b)) = (parse_numeric(mine), parse_numeric(upstream)) {
        return match a.cmp(&b) {
            Ordering::Less => VersionComparison::UpstreamNewer,
            Ordering::Greater => VersionComparison::MyNewer,
            Ordering::Equal => VersionComparison::Same,
        };
    }

    VersionComparison::Unknown
}

fn parse_semver(s: &str) -> Option<semver::Version> {
    let clean = s.trim_start_matches('v');
    semver::Version::parse(clean).ok().or_else(|| {
        let parts: Vec<&str> = clean.split('-').collect();
        semver::Version::parse(parts[0]).ok()
    })
}

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() >= 3 {
        let y = parts[0].parse::<i32>().ok()?;
        let m = parts[1].parse::<u32>().ok()?;
        let d = parts[2].parse::<u32>().ok()?;
        if y > 1990 && y < 2100 && m >= 1 && m <= 12 && d >= 1 && d <= 31 {
            return Some((y, m, d));
        }
    }
    None
}

fn parse_numeric(s: &str) -> Option<Vec<u64>> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let nums: Vec<u64> = parts
        .iter()
        .filter_map(|p| {
            let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().ok()
        })
        .collect();
    if nums.len() == parts.len() && !nums.is_empty() {
        Some(nums)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver() {
        assert_eq!(
            compare_versions("1.0.0", "2.0.0"),
            VersionComparison::UpstreamNewer
        );
        assert_eq!(
            compare_versions("2.1.0", "2.0.9"),
            VersionComparison::MyNewer
        );
        assert_eq!(
            compare_versions("1.2.3", "1.2.3"),
            VersionComparison::Same
        );
    }

    #[test]
    fn test_date() {
        assert_eq!(
            compare_versions("2023-01-01", "2024-06-15"),
            VersionComparison::UpstreamNewer
        );
        assert_eq!(
            compare_versions("2024-12-01", "2024-06-15"),
            VersionComparison::MyNewer
        );
    }

    #[test]
    fn test_numeric() {
        assert_eq!(
            compare_versions("20", "21"),
            VersionComparison::UpstreamNewer
        );
    }

    #[test]
    fn test_unknown() {
        assert_eq!(
            compare_versions("abc", "xyz"),
            VersionComparison::Unknown
        );
    }
}
