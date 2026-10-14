use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use idna::Config;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use url::Url;

const WEB_PROXY_PATH_SECRET_MARKER: u8 = 0x70;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedProxy {
    pub proxy_type: i32,
    pub address: String,
    pub port: i32,
    pub user: String,
    pub password: String,
    pub secret: String,
}

pub fn parse_proxy_uri(raw: &str) -> Option<ParsedProxy> {
    let lower = raw.to_ascii_lowercase();
    let (proxy_type, query) = if lower.starts_with("http://") || lower.starts_with("https://") {
        let url = Url::parse(raw).ok()?;
        let host = url.host_str()?.to_ascii_lowercase();
        if !matches!(host.as_str(), "telegram.me" | "t.me" | "telegram.dog") {
            return None;
        }
        let path = percent_decode_path(url.path());
        let proxy_type = if path.starts_with("/socks") {
            0
        } else if path.starts_with("/proxy") {
            1
        } else if path.starts_with("/webproxy") {
            2
        } else {
            return None;
        };
        (proxy_type, url.query()?.to_owned())
    } else if lower.starts_with("tg://") || lower.starts_with("tg:") {
        let proxy_type = if lower.starts_with("tg://socks") || lower.starts_with("tg:socks") {
            0
        } else if lower.starts_with("tg://proxy") || lower.starts_with("tg:proxy") {
            1
        } else if lower.starts_with("tg://webproxy") || lower.starts_with("tg:webproxy") {
            2
        } else {
            return None;
        };
        let query = raw.split_once('?')?.1.to_owned();
        (proxy_type, query)
    } else {
        return None;
    };

    let get = |name: &str| get_query_parameter(&query, name);

    let mut address = get("server").or_else(|| get("host")).unwrap_or_default();
    if proxy_type != 2 && check_host_for_punycode(&address) {
        if let Some(ascii) = java_idn_to_ascii(&address, false) {
            address = ascii;
        }
    }

    let port = get("port")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);
    let user = get("user").unwrap_or_default();
    let password = get("pass").unwrap_or_default();
    let mut secret = get("secret").unwrap_or_default();

    if proxy_type == 2 {
        address = normalize_web_address(&address);
        if has_web_proxy_path(&address) {
            secret = decode_marked_web_proxy_secret(&secret)?;
        } else if !is_valid_web_proxy_secret(&secret) {
            return None;
        }
        if !is_valid_web_address(&address) {
            return None;
        }
    }

    Some(ParsedProxy {
        proxy_type,
        address,
        port,
        user,
        password,
        secret,
    })
}

#[allow(deprecated)]
fn java_idn_to_ascii(host: &str, std3: bool) -> Option<String> {
    Config::default()
        .use_std3_ascii_rules(std3)
        .transitional_processing(true)
        .check_hyphens(std3)
        .verify_dns_length(true)
        .to_ascii(host)
        .ok()
}

fn check_host_for_punycode(value: &str) -> bool {
    let mut has_latin = false;
    let mut has_non_latin = false;
    for ch in value.chars() {
        if ch == '.' || ch == '-' || ch == '/' || ch == '+' || ch.is_ascii_digit() {
            continue;
        }
        if ch.is_ascii_alphabetic() {
            has_latin = true;
        } else {
            has_non_latin = true;
        }
        if has_latin && has_non_latin {
            return true;
        }
    }
    false
}

pub fn normalize_web_address(address: &str) -> String {
    if address.is_empty() {
        return String::new();
    }
    let (host, path) = match address.split_once('/') {
        Some((host, path)) => (host, Some(path)),
        None => (address, None),
    };
    if host.is_empty()
        || host.contains(':')
        || host.contains('?')
        || host.contains('#')
        || path.is_some_and(|path| !is_valid_web_proxy_path(path))
    {
        return address.to_owned();
    }

    match java_idn_to_ascii(host, true) {
        Some(host) => {
            let host = host.to_ascii_lowercase();
            match path {
                Some(path) => format!("{host}/{path}"),
                None => host,
            }
        }
        None => address.to_owned(),
    }
}

pub fn is_valid_web_address(address: &str) -> bool {
    if address.is_empty() {
        return false;
    }
    let (host, path) = match address.split_once('/') {
        Some((host, path)) => (host, Some(path)),
        None => (address, None),
    };
    if host.is_empty()
        || host.contains(':')
        || host.contains('?')
        || host.contains('#')
        || path.is_some_and(|path| !is_valid_web_proxy_path(path))
    {
        return false;
    }
    java_idn_to_ascii(host, true)
        .map(|ascii| host == ascii.to_ascii_lowercase())
        .unwrap_or(false)
}

fn has_web_proxy_path(address: &str) -> bool {
    address.contains('/')
}

fn get_query_parameter(query: &str, name: &str) -> Option<String> {
    for pair in query.split('&') {
        let (raw_name, raw_value) = pair
            .split_once('=')
            .map_or((pair, ""), |(key, value)| (key, value));
        if raw_name == name {
            return Some(percent_decode_uri_component(raw_value));
        }
    }
    None
}

fn percent_decode_uri_component(input: &str) -> String {
    percent_decode(input, true)
}

fn percent_decode_path(input: &str) -> String {
    percent_decode(input, false)
}

fn percent_decode(input: &str, plus_as_space: bool) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if plus_as_space && bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_nibble(bytes[i + 1]), hex_nibble(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn path_regex() -> &'static Regex {
    static PATH: OnceLock<Regex> = OnceLock::new();
    PATH.get_or_init(|| {
        Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_-]*(/[A-Za-z0-9][A-Za-z0-9_-]*)*$").unwrap()
    })
}

fn marked_secret_regex() -> &'static Regex {
    static SECRET: OnceLock<Regex> = OnceLock::new();
    SECRET.get_or_init(|| Regex::new(r"^[A-Za-z0-9_-]+$").unwrap())
}

pub fn is_valid_web_proxy_path(path: &str) -> bool {
    path.len() <= 128 && path_regex().is_match(path)
}

pub fn is_valid_web_proxy_secret(secret: &str) -> bool {
    let bytes = secret.as_bytes();
    if bytes.len() == 32 {
        return bytes.iter().all(u8::is_ascii_hexdigit);
    }
    bytes.len() == 34
        && bytes[0].eq_ignore_ascii_case(&b'd')
        && bytes[1].eq_ignore_ascii_case(&b'd')
        && bytes[2..].iter().all(u8::is_ascii_hexdigit)
}

pub fn decode_marked_web_proxy_secret(secret: &str) -> Option<String> {
    if secret.is_empty() || !marked_secret_regex().is_match(secret) {
        return None;
    }
    let marked = URL_SAFE_NO_PAD.decode(secret).ok()?;
    if marked.len() < 17 || marked.first().copied()? != WEB_PROXY_PATH_SECRET_MARKER {
        return None;
    }
    if URL_SAFE_NO_PAD.encode(&marked) != secret {
        return None;
    }
    let decoded = bytes_to_hex(&marked[1..]);
    is_valid_web_proxy_secret(&decoded).then_some(decoded)
}

pub fn encode_marked_web_proxy_secret(secret: &str) -> String {
    if !is_valid_web_proxy_secret(secret) {
        return secret.to_owned();
    }
    let Some(decoded) = hex_to_bytes(secret) else {
        return secret.to_owned();
    };
    let mut marked = Vec::with_capacity(decoded.len() + 1);
    marked.push(WEB_PROXY_PATH_SECRET_MARKER);
    marked.extend_from_slice(&decoded);
    URL_SAFE_NO_PAD.encode(marked)
}

fn hex_to_bytes(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = hex(bytes[i])?;
        let lo = hex(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn bytes_to_hex(value: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_socks_link() {
        let proxy = parse_proxy_uri("https://t.me/socks?server=example.com&port=1080&user=u&pass=p").unwrap();
        assert_eq!(proxy.proxy_type, 0);
        assert_eq!(proxy.address, "example.com");
        assert_eq!(proxy.port, 1080);
        assert_eq!(proxy.user, "u");
    }

    #[test]
    fn http_proxy_type_uses_decoded_android_path() {
        let proxy = parse_proxy_uri(
            "https://t.me/%73ocks?server=example.com&port=1080"
        ).unwrap();
        assert_eq!(proxy.proxy_type, 0);
        assert_eq!(proxy.address, "example.com");
    }

    #[test]
    fn query_values_decode_plus_like_android_uri() {
        let proxy = parse_proxy_uri(
            "tg://socks?server=example.com&port=1080&user=a+b&pass=x%2By"
        ).unwrap();
        assert_eq!(proxy.user, "a b");
        assert_eq!(proxy.password, "x+y");
    }

    #[test]
    fn non_web_proxy_keeps_ascii_case_without_mixed_script_punycode() {
        let proxy = parse_proxy_uri("tg://socks?server=EXAMPLE.COM&port=1080").unwrap();
        assert_eq!(proxy.address, "EXAMPLE.COM");
    }

    #[test]
    fn non_web_proxy_keeps_pure_non_latin_host_like_legacy_code() {
        let proxy = parse_proxy_uri("tg://socks?server=%E4%BE%8B%E3%81%88.%E3%83%86%E3%82%B9%E3%83%88&port=1080").unwrap();
        assert_eq!(proxy.address, "例え.テスト");
    }

    #[test]
    fn non_web_proxy_punycodes_mixed_latin_non_latin_host() {
        let proxy = parse_proxy_uri("tg://socks?server=a%E4%BE%8B%E3%81%88.com&port=1080").unwrap();
        assert!(proxy.address.is_ascii());
        assert_ne!(proxy.address, "a例え.com");
    }

    #[test]
    fn parses_tg_proxy_link() {
        let proxy = parse_proxy_uri("tg://proxy?server=example.com&port=443&secret=abcdef").unwrap();
        assert_eq!(proxy.proxy_type, 1);
        assert_eq!(proxy.address, "example.com");
        assert_eq!(proxy.port, 443);
        assert_eq!(proxy.secret, "abcdef");
    }

    #[test]
    fn parses_web_proxy_path_with_marked_secret() {
        let secret = "0123456789abcdef0123456789abcdef";
        let marked = encode_marked_web_proxy_secret(secret);
        let raw = format!("https://t.me/webproxy?server=EXAMPLE.COM/path_a&secret={marked}");
        let proxy = parse_proxy_uri(&raw).unwrap();
        assert_eq!(proxy.proxy_type, 2);
        assert_eq!(proxy.address, "example.com/path_a");
        assert_eq!(proxy.secret, secret);
        assert_eq!(proxy.port, 0);
    }

    #[test]
    fn invalid_utf8_boundary_secret_is_rejected_without_panicking() {
        let secret = format!("€{}", "a".repeat(31));
        assert_eq!(secret.len(), 34);
        assert!(!is_valid_web_proxy_secret(&secret));
    }

    #[test]
    fn marked_web_secret_roundtrips() {
        let secret = "0123456789abcdef0123456789abcdef";
        let marked = encode_marked_web_proxy_secret(secret);
        assert_eq!(decode_marked_web_proxy_secret(&marked).as_deref(), Some(secret));
    }

    #[test]
    fn web_host_enforces_std3_ascii_rules() {
        assert_eq!(normalize_web_address("_bad.example/path"), "_bad.example/path");
        assert!(!is_valid_web_address("_bad.example/path"));
    }

    #[test]
    fn normalizes_web_host() {
        assert_eq!(normalize_web_address("EXAMPLE.COM/path_a"), "example.com/path_a");
    }
}
