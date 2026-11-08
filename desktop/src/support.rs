use regex::Regex;
use std::sync::LazyLock;

pub fn web_links(text: &str) -> Vec<String> {
    static LINKS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^\s<>"']+"#).unwrap());
    let mut urls = Vec::new();
    for m in LINKS.find_iter(text) {
        let url = m
            .as_str()
            .trim_end_matches(['.', ',', ';', '!', '?', ')', ']']);
        if reqwest::Url::parse(url).is_ok_and(|u| u.host_str().is_some())
            && !urls.iter().any(|s| s == url)
        {
            urls.push(url.to_string());
        }
    }
    urls
}

pub fn compress_selection(mut indices: Vec<usize>) -> String {
    indices.sort_unstable();
    indices.dedup();
    indices.retain(|n| *n > 0);
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < indices.len() {
        let start = indices[i];
        let mut end = start;
        i += 1;
        while i < indices.len() && indices[i] == end + 1 {
            end = indices[i];
            i += 1;
        }
        ranges.push(if start == end {
            start.to_string()
        } else {
            format!("{start}:{end}")
        });
    }
    ranges.join(",")
}

pub fn checksum_for(name: &str, text: &str) -> Option<String> {
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let hash = fields.next().unwrap_or_default();
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        match fields.next() {
            Some(file) if file.trim_start_matches('*') == name => return Some(hash.to_lowercase()),
            None if text.lines().count() == 1 => return Some(hash.to_lowercase()),
            _ => (),
        }
    }
    None
}

pub fn newer(candidate: &str, current: &str) -> bool {
    let parse = |s: &str| {
        s.trim_start_matches('v')
            .split('.')
            .map(|n| n.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let mut a = parse(candidate);
    let mut b = parse(current);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a > b
}

pub fn bytes(n: f64) -> String {
    if n < 1000.0 {
        return format!("{n:.0} bytes");
    }
    for (unit, divisor) in [("GB", 1e9), ("MB", 1e6), ("KB", 1e3)] {
        if n >= divisor {
            return format!("{:.1} {unit}", n / divisor);
        }
    }
    String::new()
}
pub fn duration(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// POSIX shell on Unix, PowerShell on Windows. Execution always uses argv directly.
pub fn command_line(args: &[String], windows: bool) -> String {
    if !windows {
        return shell_words::join(args);
    }
    format!(
        "& {}",
        args.iter()
            .map(|arg| format!("'{}'", arg.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(" ")
    )
}
