use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendlyError {
    pub kind: String,
    pub message: String,
}
impl FriendlyError {
    pub fn from_raw(raw: &str) -> Self {
        let text = raw.to_lowercase();
        let cases: &[(&[&str], &str, &str)] = &[
            (
                &[
                    "cookies database",
                    "decrypt",
                    "could not copy",
                    "failed to load cookies",
                    "keyring",
                ],
                "browserCookies",
                "Couldn't read your browser sign-in. Try a different browser.",
            ),
            (
                &[
                    "not a bot",
                    "confirm your age",
                    "age-restricted",
                    "members-only",
                    "join this channel",
                    "private video",
                    "login required",
                    "log in",
                    "sign in",
                    "use --cookies",
                ],
                "signInRequired",
                "Sign in on the site in your browser, then choose that browser under More.",
            ),
            (
                &["unsupported url"],
                "unsupportedLink",
                "Purrloader can't download from this link yet.",
            ),
            (
                &["is not a valid url"],
                "invalidLink",
                "That doesn't look like a link.",
            ),
            (
                &[
                    "video unavailable",
                    "this video has been removed",
                    "not available in your country",
                    "geo restrict",
                ],
                "unavailable",
                "This video is unavailable. It may be private, removed, or blocked in your country.",
            ),
            (
                &["requested format is not available"],
                "formatUnavailable",
                "That quality isn't available for this video.",
            ),
            (
                &[
                    "http error 403",
                    "http error 429",
                    "precondition check failed",
                    "nsig extraction failed",
                    "signature extraction",
                    "unable to extract",
                ],
                "blocked",
                "The site blocked the download. Try updating the helper tools.",
            ),
            (
                &[
                    "unable to download webpage",
                    "nodename nor servname",
                    "timed out",
                    "network is unreachable",
                    "connection reset",
                    "name resolution",
                    "name or service not known",
                    "getaddrinfo failed",
                    "no address associated",
                    "winerror 10060",
                    "winerror 10054",
                    "winerror 10065",
                ],
                "network",
                "Couldn't reach the site. Check your internet connection.",
            ),
            (
                &["no space left", "not enough space on the disk"],
                "diskFull",
                "Your disk is full. Free some space and try again.",
            ),
            (
                &[
                    "permission denied",
                    "operation not permitted",
                    "read-only file system",
                    "access is denied",
                ],
                "folderNotWritable",
                "Purrloader can't save to that folder.",
            ),
            (
                &["this live event will begin", "premieres in", "is_upcoming"],
                "notStarted",
                "This stream hasn't started yet.",
            ),
        ];
        for (needles, kind, message) in cases {
            if needles.iter().any(|n| text.contains(n)) {
                return Self {
                    kind: (*kind).into(),
                    message: (*message).into(),
                };
            }
        }
        let message = if raw.starts_with('[') {
            raw.split_once(": ").map(|(_, s)| s).unwrap_or(raw)
        } else {
            raw
        };
        Self {
            kind: "other".into(),
            message: if message.trim().is_empty() {
                "The download stopped unexpectedly.".into()
            } else {
                message.into()
            },
        }
    }
    pub fn fix_label(&self) -> &str {
        match self.kind.as_str() {
            "browserCookies" | "signInRequired" | "unavailable" => "Use browser sign-in",
            "blocked" | "unsupportedLink" => "Update and retry",
            "folderNotWritable" => "Choose another folder",
            "formatUnavailable" => "Use best quality",
            "notStarted" => "Wait for start",
            _ => "Retry",
        }
    }
}
