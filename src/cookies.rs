use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieEntry {
    pub domain: String,
    pub include_subdomains: bool,
    pub path: String,
    pub secure: bool,
    pub expires: i64,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieStatus {
    pub target: String,
    pub exists: bool,
    pub path: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub valid: bool,
    pub status_label: String,
    pub has_login_info: bool,
    pub has_sid: bool,
    pub expires_at: Option<i64>,
    pub days_remaining: Option<i64>,
    pub total_cookies: usize,
    pub sanitized_cookies: usize,
}

/// Core authentication cookie names required for YouTube
const YOUTUBE_CORE_AUTH_KEYS: &[&str] = &[
    "LOGIN_INFO",
    "SID",
    "HSID",
    "SSID",
    "APISID",
    "SAPISID",
    "__Secure-1PSID",
    "__Secure-3PSID",
    "__Secure-1PAPISID",
    "__Secure-3PAPISID",
    "PREF",
    "YSC",
    "VISITOR_INFO1_LIVE",
    "VISITOR_PRIVACY_METADATA",
    "wide",
];

/// Known third-party tracker domains to discard
const TRACKER_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "google-analytics.com",
    "googlesyndication.com",
    "adservice.google.com",
    "facebook.com",
    "adnxs.com",
    "scorecardresearch.com",
    "rubiconproject.com",
    "openx.net",
    "pubmatic.com",
];

/// Returns the default cookie file path for a target
pub fn get_cookie_file_path(target: &str) -> Result<PathBuf> {
    let folder = if let Some(base_dirs) = directories::BaseDirs::new() {
        base_dirs.data_local_dir().join("VampYTD")
    } else {
        PathBuf::from(".").join(".vampytd")
    };

    if !folder.exists() {
        fs::create_dir_all(&folder)
            .with_context(|| format!("Failed to create directory: {}", folder.display()))?;
    }

    let file_name = match target.to_lowercase().as_str() {
        "hotstar" | "jhs" | "jiohotstar" => "cookies-jhs.txt",
        _ => "cookies-yt.txt",
    };

    Ok(folder.join(file_name))
}

/// Parses a single line from a Netscape HTTP Cookie File
pub fn parse_netscape_line(line: &str) -> Option<CookieEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let parts: Vec<&str> = trimmed.split('\t').collect();
    if parts.len() < 7 {
        return None;
    }

    let domain = parts[0].to_string();
    let include_subdomains = parts[1].eq_ignore_ascii_case("true");
    let path = parts[2].to_string();
    let secure = parts[3].eq_ignore_ascii_case("true");
    let expires = parts[4].parse::<i64>().unwrap_or(0);
    let name = parts[5].to_string();
    let value = parts[6].to_string();

    Some(CookieEntry {
        domain,
        include_subdomains,
        path,
        secure,
        expires,
        name,
        value,
    })
}

/// Checks if a cookie belongs to YouTube/Google domains
fn is_youtube_cookie(domain: &str) -> bool {
    let d = domain.to_lowercase();
    (d.contains("youtube.com") || d.contains("google.com") || d.contains("youtu.be"))
        && !TRACKER_DOMAINS.iter().any(|&tracker| d.contains(tracker))
}

/// Checks if a cookie belongs to Hotstar/JioHotstar domains
fn is_hotstar_cookie(domain: &str) -> bool {
    let d = domain.to_lowercase();
    (d.contains("hotstar.com") || d.contains("jiohotstar.com"))
        && !TRACKER_DOMAINS.iter().any(|&tracker| d.contains(tracker))
}

/// Sanitizes cookie content, keeping only valid authentication cookies and stripping trackers
pub fn sanitize_cookies(content: &str, target: &str) -> Result<(Vec<CookieEntry>, usize)> {
    let is_jhs = matches!(target.to_lowercase().as_str(), "hotstar" | "jhs" | "jiohotstar");
    let mut total_lines = 0;
    let mut sanitized = Vec::new();

    for line in content.lines() {
        if let Some(entry) = parse_netscape_line(line) {
            total_lines += 1;

            if is_jhs {
                if is_hotstar_cookie(&entry.domain) {
                    sanitized.push(entry);
                }
            } else {
                if is_youtube_cookie(&entry.domain) {
                    let is_core_key = YOUTUBE_CORE_AUTH_KEYS.iter().any(|&k| entry.name.eq_ignore_ascii_case(k));
                    let is_general_yt = entry.domain.contains("youtube.com");

                    // Keep core auth keys or any domain-specific youtube.com non-tracker cookie
                    if is_core_key || is_general_yt {
                        sanitized.push(entry);
                    }
                }
            }
        }
    }

    if sanitized.is_empty() {
        bail!(
            "No valid {} cookies found in the provided file. Ensure it is in Netscape format.",
            if is_jhs { "Hotstar" } else { "YouTube" }
        );
    }

    // Safety validation for YouTube: verify presence of at least one genuine session token
    if !is_jhs {
        let has_login_token = sanitized.iter().any(|c| {
            c.name.eq_ignore_ascii_case("LOGIN_INFO")
                || c.name.eq_ignore_ascii_case("SID")
                || c.name.eq_ignore_ascii_case("__Secure-1PSID")
                || c.name.eq_ignore_ascii_case("__Secure-3PSID")
        });

        if !has_login_token {
            bail!("Imported file contains YouTube cookies, but is missing core authentication tokens (LOGIN_INFO or SID). Please export while signed into your account.");
        }
    }

    Ok((sanitized, total_lines))
}

/// Imports and sanitizes cookie content, writing to the target file
pub fn import_and_save(content: &str, target: &str) -> Result<CookieStatus> {
    let dest_path = get_cookie_file_path(target)?;
    let (sanitized, total_count) = sanitize_cookies(content, target)?;

    let mut file = File::create(&dest_path)
        .with_context(|| format!("Failed to create cookie file at {}", dest_path.display()))?;

    writeln!(file, "# Netscape HTTP Cookie File")?;
    writeln!(
        file,
        "# Sanitized and generated by VampYTD on {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(file, "# Edit at your own risk.")?;
    writeln!(file)?;

    for c in &sanitized {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            c.domain,
            if c.include_subdomains { "TRUE" } else { "FALSE" },
            c.path,
            if c.secure { "TRUE" } else { "FALSE" },
            c.expires,
            c.name,
            c.value
        )?;
    }
    file.flush()?;

    let mut status = inspect_cookie_file(&dest_path, target)?;
    status.total_cookies = total_count;
    status.sanitized_cookies = sanitized.len();
    Ok(status)
}

/// Inspects an existing cookie file and computes its health status
pub fn inspect_cookie_file(path: &Path, target: &str) -> Result<CookieStatus> {
    if !path.exists() {
        return Ok(CookieStatus {
            target: target.to_string(),
            exists: false,
            path: Some(path.to_string_lossy().to_string()),
            file_size_bytes: None,
            valid: false,
            status_label: "Not Found".to_string(),
            has_login_info: false,
            has_sid: false,
            expires_at: None,
            days_remaining: None,
            total_cookies: 0,
            sanitized_cookies: 0,
        });
    }

    let metadata = fs::metadata(path)?;
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut count = 0;
    let mut has_login_info = false;
    let mut has_sid = false;
    let mut latest_expiry: i64 = 0;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    for line in reader.lines() {
        if let Ok(l) = line {
            if let Some(entry) = parse_netscape_line(&l) {
                count += 1;
                if entry.name.eq_ignore_ascii_case("LOGIN_INFO") {
                    has_login_info = true;
                }
                if entry.name.eq_ignore_ascii_case("SID")
                    || entry.name.eq_ignore_ascii_case("__Secure-1PSID")
                    || entry.name.eq_ignore_ascii_case("__Secure-3PSID")
                {
                    has_sid = true;
                }
                if entry.expires > latest_expiry {
                    latest_expiry = entry.expires;
                }
            }
        }
    }

    let is_jhs = matches!(target.to_lowercase().as_str(), "hotstar" | "jhs" | "jiohotstar");
    let is_auth_valid = if is_jhs {
        count > 0
    } else {
        has_login_info || has_sid
    };

    let expires_at = if latest_expiry > 0 {
        Some(latest_expiry)
    } else {
        None
    };

    let days_remaining = expires_at.map(|exp| (exp - now) / 86400);

    let (valid, status_label) = if !is_auth_valid {
        (false, "Invalid (Missing Session)".to_string())
    } else if let Some(days) = days_remaining {
        if days < 0 {
            (false, "Expired".to_string())
        } else if days <= 7 {
            (true, format!("Expiring Soon ({}d)", days))
        } else {
            (true, format!("Active ({}d remaining)", days))
        }
    } else {
        (true, "Active".to_string())
    };

    Ok(CookieStatus {
        target: target.to_string(),
        exists: true,
        path: Some(path.to_string_lossy().to_string()),
        file_size_bytes: Some(metadata.len()),
        valid,
        status_label,
        has_login_info,
        has_sid,
        expires_at,
        days_remaining,
        total_cookies: count,
        sanitized_cookies: count,
    })
}

/// Returns the status of a specific target
pub fn get_cookie_status(target: &str) -> Result<CookieStatus> {
    let path = get_cookie_file_path(target)?;
    inspect_cookie_file(&path, target)
}

/// Deletes the cookie file for a target
pub fn clear_cookie(target: &str) -> Result<bool> {
    let path = get_cookie_file_path(target)?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete cookie file: {}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_netscape_line_valid() {
        let line = ".youtube.com\tTRUE\t/\tTRUE\t1780000000\tLOGIN_INFO\tAFmmF2cwRQIh...";
        let entry = parse_netscape_line(line).expect("Should parse valid line");
        assert_eq!(entry.domain, ".youtube.com");
        assert!(entry.include_subdomains);
        assert_eq!(entry.path, "/");
        assert!(entry.secure);
        assert_eq!(entry.expires, 1780000000);
        assert_eq!(entry.name, "LOGIN_INFO");
        assert_eq!(entry.value, "AFmmF2cwRQIh...");
    }

    #[test]
    fn test_parse_netscape_line_comment_or_empty() {
        assert!(parse_netscape_line("# This is a comment").is_none());
        assert!(parse_netscape_line("").is_none());
        assert!(parse_netscape_line("   ").is_none());
    }

    #[test]
    fn test_sanitize_cookies_strips_trackers() {
        let input = r#"
# Netscape HTTP Cookie File
.youtube.com	TRUE	/	TRUE	1893456000	LOGIN_INFO	test_login_info
.youtube.com	TRUE	/	TRUE	1893456000	SID	test_sid
.doubleclick.net	TRUE	/	FALSE	1893456000	IDE	ad_tracker_value
.google-analytics.com	TRUE	/	FALSE	1893456000	_ga	ga_tracker_value
"#;
        let (sanitized, total) = sanitize_cookies(input, "youtube").expect("Sanitize should succeed");
        assert_eq!(total, 4);
        assert_eq!(sanitized.len(), 2);
        assert!(sanitized.iter().any(|c| c.name == "LOGIN_INFO"));
        assert!(sanitized.iter().any(|c| c.name == "SID"));
        assert!(!sanitized.iter().any(|c| c.domain.contains("doubleclick")));
        assert!(!sanitized.iter().any(|c| c.domain.contains("google-analytics")));
    }

    #[test]
    fn test_sanitize_cookies_rejects_missing_auth() {
        let input = r#"
.youtube.com	TRUE	/	FALSE	1893456000	SOME_RANDOM_COOKIE	random_val
"#;
        let res = sanitize_cookies(input, "youtube");
        assert!(res.is_err(), "Should reject cookies missing LOGIN_INFO / SID");
    }
}
