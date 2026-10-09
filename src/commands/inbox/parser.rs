use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedOtp {
    pub code: String,
    pub service: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLink {
    pub url: String,
    pub link_type: String, // "verification" | "reset_password" | "magic_link" | "action"
}

#[derive(Debug, Clone, Default)]
pub struct ParsedEmailContent {
    pub otps: Vec<ParsedOtp>,
    pub links: Vec<ParsedLink>,
    pub detected_service: Option<String>,
}

// Pre-compiled regex patterns for maximum throughput
static CONTEXT_OTP_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Vietnamese keywords
        Regex::new(r#"(?i)(?:mã\s*(?:xác\s*nhận|xác\s*thực|otp|bảo\s*mật|đăng\s*nhập|kích\s*hoạt|khôi\s*phục|của\s*bạn\s*là))[^0-9\n]{0,25}([0-9]{4,8})\b"#).unwrap(),
        Regex::new(r#"(?i)\b([0-9]{4,8})[^0-9\n]{1,25}(?:mã\s*(?:xác\s*nhận|xác\s*thực|otp|bảo\s*mật|đăng\s*nhập|kích\s*hoạt)|otp)"#).unwrap(),

        // English keywords
        Regex::new(r#"(?i)(?:verification|confirmation|security|authorization|login|activation|passcode|one-time\s+password|otp|pin)[^0-9\n]{0,30}([0-9]{4,8})\b"#).unwrap(),
        Regex::new(r#"(?i)(?:your|the)\s+(?:single-use\s+code|code)[^0-9\n]{0,20}([0-9]{4,8})\b"#).unwrap(),
        Regex::new(r#"(?i)(?:enter|use|type)\s+(?:this\s+)?(?:code|otp|password|one-time)[^0-9\n]{0,30}([0-9]{4,8})\b"#).unwrap(),
        Regex::new(r#"(?i)\b([0-9]{4,8})[^0-9\n]{1,30}(?:is\s+your|is\s+the|verification|confirmation|security|one-time|otp)"#).unwrap(),

        // Platform specific prefixes like G-123456 or FB-12345
        Regex::new(r#"(?i)\b(?:G|FB|TG|DISCORD|TT)-([0-9]{4,8})\b"#).unwrap(),

        // Bold / Large prominent code blocks e.g. <b style="font-size: 24px">123456</b> or > 123456 <
        Regex::new(r#"(?i)<(?:b|strong|h1|h2|h3)[^>]*>\s*([0-9]{4,8})\s*</(?:b|strong|h1|h2|h3)>"#).unwrap(),
        Regex::new(r#"(?i)(?:class|id)=["'][^"']*(?:code|otp|token|digit)[^"']*["'][^>]*>\s*([0-9]{4,8})\s*<"#).unwrap(),
    ]
});

// Fallback pattern for isolated 4-8 digit standalone numbers surrounded by whitespace
static STANDALONE_OTP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b([0-9]{4,8})\b"#).unwrap()
});

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?://[^\s<>"'{}|\\^`\[\]]+"#).unwrap()
});

pub struct EmailParser;

impl EmailParser {
    /// Detect service identity from sender email address, headers, or subject
    pub fn detect_service(sender: &str, subject: Option<&str>) -> Option<String> {
        let sender_lower = sender.to_ascii_lowercase();
        let subject_lower = subject.map(|s| s.to_ascii_lowercase()).unwrap_or_default();
        let combined = format!("{} {}", sender_lower, subject_lower);

        if combined.contains("facebook") || combined.contains("meta.com") {
            Some("Facebook".to_string())
        } else if combined.contains("google") || combined.contains("gmail") || combined.contains("youtube") {
            Some("Google".to_string())
        } else if combined.contains("tiktok") {
            Some("TikTok".to_string())
        } else if combined.contains("telegram") {
            Some("Telegram".to_string())
        } else if combined.contains("discord") {
            Some("Discord".to_string())
        } else if combined.contains("shopee") {
            Some("Shopee".to_string())
        } else if combined.contains("lazada") {
            Some("Lazada".to_string())
        } else if combined.contains("amazon") || combined.contains("aws") {
            Some("Amazon".to_string())
        } else if combined.contains("microsoft") || combined.contains("live.com") || combined.contains("outlook.com") {
            Some("Microsoft".to_string())
        } else if combined.contains("twitter") || combined.contains("x.com") {
            Some("X".to_string())
        } else if combined.contains("openai") || combined.contains("chatgpt") {
            Some("OpenAI".to_string())
        } else if combined.contains("github") {
            Some("GitHub".to_string())
        } else if combined.contains("apple") || combined.contains("icloud") {
            Some("Apple".to_string())
        } else if combined.contains("steam") || combined.contains("valvesoftware") {
            Some("Steam".to_string())
        } else if combined.contains("netflix") {
            Some("Netflix".to_string())
        } else {
            // Extract domain name from sender as fallback
            if let Some(domain_part) = sender.split('@').nth(1) {
                let clean_domain = domain_part.trim_matches(|c| c == '>' || c == ' ');
                let parts: Vec<&str> = clean_domain.split('.').collect();
                if parts.len() >= 2 {
                    let name = parts[parts.len() - 2];
                    if name.len() >= 3 && !["mail", "email", "noreply", "mailer"].contains(&name) {
                        return Some(name[0..1].to_uppercase() + &name[1..]);
                    }
                }
            }
            None
        }
    }

    /// Extract OTP codes from subject, body text, or HTML content
    pub fn extract_otps(
        subject: Option<&str>,
        body_text: Option<&str>,
        body_html: Option<&str>,
        service: Option<&str>,
    ) -> Vec<ParsedOtp> {
        let mut results = Vec::new();
        let mut seen_codes = std::collections::HashSet::new();

        let sources = [
            (subject.unwrap_or_default(), 1.0f32),
            (body_text.unwrap_or_default(), 0.9f32),
            (body_html.unwrap_or_default(), 0.8f32),
        ];

        // 1. High-confidence extraction with contextual regexes
        for (content, base_confidence) in &sources {
            if content.is_empty() {
                continue;
            }

            for pattern in CONTEXT_OTP_PATTERNS.iter() {
                for caps in pattern.captures_iter(content) {
                    if let Some(matched) = caps.get(1) {
                        let code = matched.as_str().trim();
                        // Ignore common non-OTP sequences (like 1234, 0000, 2026, 2025)
                        if is_likely_otp(code) && seen_codes.insert(code.to_string()) {
                            results.push(ParsedOtp {
                                code: code.to_string(),
                                service: service.map(String::from),
                                confidence: *base_confidence,
                            });
                        }
                    }
                }
            }
        }

        // 2. If no OTP was found via contextual patterns, search subject for any prominent isolated 6-digit number
        if results.is_empty() {
            if let Some(subj) = subject {
                for caps in STANDALONE_OTP_REGEX.captures_iter(subj) {
                    if let Some(matched) = caps.get(1) {
                        let code = matched.as_str().trim();
                        if (code.len() == 6 || code.len() == 4) && is_likely_otp(code) && seen_codes.insert(code.to_string()) {
                            results.push(ParsedOtp {
                                code: code.to_string(),
                                service: service.map(String::from),
                                confidence: 0.75f32,
                            });
                        }
                    }
                }
            }
        }

        results
    }

    /// Extract action, verification, and reset password links
    pub fn extract_links(body_text: Option<&str>, body_html: Option<&str>) -> Vec<ParsedLink> {
        let mut results = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        let full_content = format!(
            "{}\n{}",
            body_text.unwrap_or_default(),
            body_html.unwrap_or_default()
        );

        for mat in URL_REGEX.find_iter(&full_content) {
            let mut url = mat.as_str().to_string();

            // Trim trailing punctuation and HTML entities
            url = url.trim_end_matches(['.', ',', ';', ')', ']', '>', '"', '\'']).to_string();

            if is_actionable_link(&url) && seen_urls.insert(url.clone()) {
                let link_type = classify_link(&url);
                results.push(ParsedLink { url, link_type });
            }
        }

        results
    }

    /// Complete email content parser
    pub fn parse(
        sender: &str,
        subject: Option<&str>,
        body_text: Option<&str>,
        body_html: Option<&str>,
    ) -> ParsedEmailContent {
        let detected_service = Self::detect_service(sender, subject);
        let otps = Self::extract_otps(
            subject,
            body_text,
            body_html,
            detected_service.as_deref(),
        );
        let links = Self::extract_links(body_text, body_html);

        ParsedEmailContent {
            otps,
            links,
            detected_service,
        }
    }
}

fn is_likely_otp(code: &str) -> bool {
    let len = code.len();
    if !(4..=8).contains(&len) {
        return false;
    }

    // Filter out common years
    if code == "2024" || code == "2025" || code == "2026" || code == "2027" || code == "1999" || code == "2000" {
        return false;
    }

    // Filter out repetitive trivial digits (e.g. 0000, 1111)
    let first = code.chars().next().unwrap();
    if code.chars().all(|c| c == first) {
        return false;
    }

    true
}

fn is_actionable_link(url: &str) -> bool {
    let url_lower = url.to_ascii_lowercase();

    // Ignore assets, images, unsubscribe, policies, static CDNs
    if url_lower.ends_with(".png")
        || url_lower.ends_with(".jpg")
        || url_lower.ends_with(".jpeg")
        || url_lower.ends_with(".gif")
        || url_lower.ends_with(".svg")
        || url_lower.ends_with(".css")
        || url_lower.ends_with(".js")
    {
        return false;
    }

    // Ignore tracking pixels & junk
    let ignored_keywords = [
        "unsubscribe",
        "optout",
        "privacy",
        "terms",
        "help",
        "support",
        "faq",
        "blog",
        "play.google.com",
        "apps.apple.com",
        "twitter.com",
        "facebook.com/policies",
        "instagram.com",
        "linkedin.com",
    ];

    for keyword in ignored_keywords {
        if url_lower.contains(keyword) {
            return false;
        }
    }

    true
}

fn classify_link(url: &str) -> String {
    let url_lower = url.to_ascii_lowercase();

    if url_lower.contains("reset")
        || url_lower.contains("password")
        || url_lower.contains("forgot")
        || url_lower.contains("recover")
        || url_lower.contains("khoi-phuc")
    {
        "reset_password".to_string()
    } else if url_lower.contains("verify")
        || url_lower.contains("confirm")
        || url_lower.contains("activate")
        || url_lower.contains("validation")
        || url_lower.contains("xac-nhan")
        || url_lower.contains("kich-hoat")
    {
        "verification".to_string()
    } else if url_lower.contains("magic")
        || url_lower.contains("one-time-token")
        || url_lower.contains("magic-link")
        || url_lower.contains("signin")
    {
        "magic_link".to_string()
    } else {
        "action".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_service() {
        assert_eq!(EmailParser::detect_service("security@facebookmail.com", None), Some("Facebook".to_string()));
        assert_eq!(EmailParser::detect_service("no-reply@accounts.google.com", None), Some("Google".to_string()));
        assert_eq!(EmailParser::detect_service("verify@tiktok.com", None), Some("TikTok".to_string()));
        assert_eq!(EmailParser::detect_service("login@telegram.org", None), Some("Telegram".to_string()));
        assert_eq!(EmailParser::detect_service("noreply@discord.com", None), Some("Discord".to_string()));
        assert_eq!(EmailParser::detect_service("noreply@shopee.vn", None), Some("Shopee".to_string()));
        assert_eq!(EmailParser::detect_service("account-update@amazon.com", None), Some("Amazon".to_string()));
        assert_eq!(EmailParser::detect_service("account-security-noreply@accountprotection.microsoft.com", None), Some("Microsoft".to_string()));
        assert_eq!(EmailParser::detect_service("verify@x.com", None), Some("X".to_string()));
        assert_eq!(EmailParser::detect_service("noreply@tm.openai.com", None), Some("OpenAI".to_string()));
    }

    #[test]
    fn test_extract_otp_vietnamese_templates() {
        // Template 1: Mã xác nhận của bạn là 618294
        let p1 = EmailParser::extract_otps(
            Some("Shopee: Mã xác nhận"),
            Some("Mã xác nhận của bạn là: 618294. Mã này có hiệu lực trong 5 phút."),
            None,
            Some("Shopee"),
        );
        assert_eq!(p1.len(), 1);
        assert_eq!(p1[0].code, "618294");

        // Template 2: 739102 là mã xác thực tài khoản
        let p2 = EmailParser::extract_otps(
            None,
            Some("739102 là mã xác thực tài khoản Facebook của bạn."),
            None,
            Some("Facebook"),
        );
        assert_eq!(p2.len(), 1);
        assert_eq!(p2[0].code, "739102");

        // Template 3: Mã OTP đăng nhập: 4920
        let p3 = EmailParser::extract_otps(
            None,
            Some("Mã OTP đăng nhập: 4920"),
            None,
            None,
        );
        assert_eq!(p3.len(), 1);
        assert_eq!(p3[0].code, "4920");

        // Template 4: Mã kích hoạt tài khoản là 92837482
        let p4 = EmailParser::extract_otps(
            None,
            Some("Mã kích hoạt tài khoản của bạn là 92837482."),
            None,
            None,
        );
        assert_eq!(p4.len(), 1);
        assert_eq!(p4[0].code, "92837482");
    }

    #[test]
    fn test_extract_otp_english_templates() {
        // Google template: G-482910 is your Google verification code
        let p1 = EmailParser::extract_otps(
            Some("Your Google verification code is 482910"),
            Some("G-482910 is your Google verification code. Do not share it."),
            None,
            Some("Google"),
        );
        assert!(p1.iter().any(|o| o.code == "482910"));

        // Discord template: Your Discord verification code is: 582910
        let p2 = EmailParser::extract_otps(
            None,
            Some("Your Discord verification code is: 582910"),
            None,
            Some("Discord"),
        );
        assert_eq!(p2.len(), 1);
        assert_eq!(p2[0].code, "582910");

        // Amazon template: 284910 is your Amazon OTP
        let p3 = EmailParser::extract_otps(
            Some("Amazon security code"),
            Some("Enter this one-time password to continue: 284910"),
            None,
            Some("Amazon"),
        );
        assert_eq!(p3.len(), 1);
        assert_eq!(p3[0].code, "284910");

        // Microsoft template
        let p4 = EmailParser::extract_otps(
            None,
            Some("Security code: 829471. If you didn't request this, ignore."),
            None,
            Some("Microsoft"),
        );
        assert_eq!(p4.len(), 1);
        assert_eq!(p4[0].code, "829471");
    }

    #[test]
    fn test_extract_otp_html_prominent_tags() {
        let html = r#"
            <div class="container">
                <p>Hello user,</p>
                <div class="code-container">
                    <h2 style="letter-spacing: 5px;">940183</h2>
                </div>
            </div>
        "#;
        let p = EmailParser::extract_otps(None, None, Some(html), None);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].code, "940183");
    }

    #[test]
    fn test_extract_links_classification_and_filtering() {
        let text = r#"
            Click here to verify: https://example.com/auth/verify?token=abc123xyz
            Or reset password: https://example.com/account/reset-password?id=999
            To unsubscribe: https://example.com/unsubscribe?user=me
            Terms of service: https://example.com/legal/terms
        "#;

        let links = EmailParser::extract_links(Some(text), None);
        assert_eq!(links.len(), 2);

        let verify_link = links.iter().find(|l| l.link_type == "verification");
        assert!(verify_link.is_some());
        assert_eq!(verify_link.unwrap().url, "https://example.com/auth/verify?token=abc123xyz");

        let reset_link = links.iter().find(|l| l.link_type == "reset_password");
        assert!(reset_link.is_some());
        assert_eq!(reset_link.unwrap().url, "https://example.com/account/reset-password?id=999");
    }

    #[test]
    fn test_full_parse_roundtrip() {
        let sender = "security@facebookmail.com";
        let subject = "Your Facebook security code is 741852";
        let body = "Hi, someone tried to log into your account. Enter code: 741852 or click https://facebook.com/confirm?token=fb9988 to verify.";

        let res = EmailParser::parse(sender, Some(subject), Some(body), None);
        assert_eq!(res.detected_service.as_deref(), Some("Facebook"));
        assert!(!res.otps.is_empty());
        assert_eq!(res.otps[0].code, "741852");
        assert!(!res.links.is_empty());
        assert_eq!(res.links[0].url, "https://facebook.com/confirm?token=fb9988");
        assert_eq!(res.links[0].link_type, "verification");
    }
}
