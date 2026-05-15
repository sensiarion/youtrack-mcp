use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use chrono_tz::Tz;

use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub token: String,
    pub timezone: Tz,
    pub default_project: Option<String>,
    pub holidays: HashSet<NaiveDate>,
    pub pre_holidays: HashSet<NaiveDate>,
    pub user_aliases: HashMap<String, String>,
    pub download_dir: Option<String>,
}

fn parse_dates(raw: &str) -> HashSet<NaiveDate> {
    raw.split(',')
        .filter_map(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .collect()
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("YOUTRACK_URL")
            .map_err(|_| AppError::Config("YOUTRACK_URL is required".into()))?
            .trim_end_matches('/')
            .to_string();
        let token = std::env::var("YOUTRACK_TOKEN")
            .map_err(|_| AppError::Config("YOUTRACK_TOKEN is required".into()))?;
        let timezone = std::env::var("YOUTRACK_TIMEZONE")
            .ok()
            .and_then(|s| s.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::Europe::Moscow);
        let default_project = std::env::var("YOUTRACK_DEFAULT_PROJECT").ok().filter(|s| !s.is_empty());
        let holidays = std::env::var("YOUTRACK_HOLIDAYS").map(|s| parse_dates(&s)).unwrap_or_default();
        let pre_holidays =
            std::env::var("YOUTRACK_PRE_HOLIDAYS").map(|s| parse_dates(&s)).unwrap_or_default();
        let user_aliases = std::env::var("YOUTRACK_USER_ALIASES")
            .map(|s| {
                s.split(',')
                    .filter_map(|pair| {
                        let (a, l) = pair.split_once(':')?;
                        Some((a.trim().to_string(), l.trim().to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let download_dir = std::env::var("YOUTRACK_DOWNLOAD_DIR").ok().filter(|s| !s.is_empty());

        Ok(Config {
            base_url,
            token,
            timezone,
            default_project,
            holidays,
            pre_holidays,
            user_aliases,
            download_dir,
        })
    }

    /// Expand a bare numeric issue id ("123") to "PROJ-123" when a default
    /// project is configured. Ids already containing "-" pass through.
    pub fn expand_issue_id(&self, raw: &str) -> String {
        let t = raw.trim();
        match &self.default_project {
            Some(p) if !t.contains('-') => format!("{p}-{t}"),
            _ => t.to_string(),
        }
    }

    /// Map a configured alias to a real login, otherwise return the input.
    pub fn resolve_alias<'a>(&'a self, login: &'a str) -> &'a str {
        self.user_aliases.get(login).map(|s| s.as_str()).unwrap_or(login)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(default_project: Option<&str>) -> Config {
        Config {
            base_url: "https://x".into(),
            token: "t".into(),
            timezone: chrono_tz::Europe::Moscow,
            default_project: default_project.map(String::from),
            holidays: HashSet::new(),
            pre_holidays: HashSet::new(),
            user_aliases: HashMap::from([("me".to_string(), "real".to_string())]),
            download_dir: None,
        }
    }

    #[test]
    fn expands_bare_numeric_only_with_default_project() {
        assert_eq!(cfg(Some("PROJ")).expand_issue_id("123"), "PROJ-123");
        assert_eq!(cfg(Some("PROJ")).expand_issue_id("OTHER-9"), "OTHER-9");
        assert_eq!(cfg(None).expand_issue_id("123"), "123");
    }

    #[test]
    fn resolves_alias() {
        let c = cfg(None);
        assert_eq!(c.resolve_alias("me"), "real");
        assert_eq!(c.resolve_alias("someone"), "someone");
    }

    #[test]
    fn parses_csv_dates() {
        let s = parse_dates("2026-01-01, 2026-05-09 ,bad");
        assert_eq!(s.len(), 2);
        assert!(s.contains(&NaiveDate::from_ymd_opt(2026, 5, 9).unwrap()));
    }
}
