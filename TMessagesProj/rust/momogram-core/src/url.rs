const TWITTER_DOMAINS: &[&str] = &["x.com", "twitter.com"];
const TWITTER_PREVIEW_DOMAINS: &[&str] = &["vxtwitter.com", "fxtwitter.com", "fixupx.com", "fixvx.com"];
const YOUTUBE_DOMAINS: &[&str] = &["youtube.com", "youtu.be"];
const TRACKING_QUERY_PREFIXES: &[&str] = &["utm"];
const TRACKING_QUERY_POSTFIXES: &[&str] = &[
    "shid", "clid", "wtrid", "spm", "tracking_source", "__s", "__hssc", "__hstc",
    "hsCtaTracking", "msclkid", "oly_anon_id", "oly_enc_id", "rb_clickid", "s_clid",
    "wickedid",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedUrl<'a> {
    scheme: &'a str,
    host: &'a str,
    port: Option<u16>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}


pub fn is_twitter(host: &str) -> bool {
    TWITTER_DOMAINS.contains(&host)
}

pub fn is_twitter_preview(host: &str) -> bool {
    TWITTER_PREVIEW_DOMAINS.contains(&host)
}

pub fn strip_all_queries(src: &str) -> String {
    src.rsplit_once('?').map(|(head, _)| head.to_owned()).unwrap_or_else(|| src.to_owned())
}

pub fn clean_url(src: &str, patch_links: bool, custom_blacklist: &[&str]) -> String {
    let Some(parsed) = parse_url(src) else {
        return src.to_owned();
    };
    if !parsed.scheme.starts_with("http") {
        return src.to_owned();
    }

    if TWITTER_DOMAINS.contains(&parsed.host) || TWITTER_PREVIEW_DOMAINS.contains(&parsed.host) {
        let host = if patch_links { "twitter.com" } else { parsed.host };
        return format!("https://{host}{}", android_builder_path(parsed.path));
    }

    if YOUTUBE_DOMAINS.contains(&parsed.host) {
        let mut out = format!("https://{}{}", parsed.host, android_builder_path(parsed.path));
        if let Some(query) = parsed.query {
            if let Some(value) = android_get_query_parameter(query, "t") {
                out.push_str("?t=");
                out.push_str(&uri_encode_component(&value));
            }
        }
        return out;
    }

    let mut out = String::new();
    out.push_str(parsed.scheme);
    out.push_str("://");
    out.push_str(parsed.host);
    if let Some(port) = parsed.port {
        let is_default = (parsed.scheme == "http" && port == 80) || (parsed.scheme == "https" && port == 443);
        if !is_default {
            out.push(':');
            out.push_str(&port.to_string());
        }
    }
    out.push_str(&legacy_general_builder_path(parsed.path));

    if let Some(query) = parsed.query {
        let mut first = true;
        for decoded_name in query_parameter_names(query) {
            let blocked = TRACKING_QUERY_PREFIXES.iter().any(|prefix| decoded_name.starts_with(prefix))
                || TRACKING_QUERY_POSTFIXES.iter().any(|prefix| decoded_name.starts_with(prefix))
                || custom_blacklist.iter().any(|blocked| decoded_name == *blocked);
            if blocked {
                continue;
            }

            // Android's Uri.getQueryParameter() canonical-encodes the decoded
            // name before comparing it with the raw query. A non-canonical raw
            // key can therefore appear in getQueryParameterNames() while its
            // value lookup returns null.
            let decoded_value = android_get_query_parameter(query, &decoded_name);

            out.push(if first { '?' } else { '&' });
            first = false;
            out.push_str(&uri_encode_component(&decoded_name));
            out.push('=');
            out.push_str(&uri_encode_component(decoded_value.as_deref().unwrap_or("null")));
        }
    }

    if let Some(fragment) = parsed.fragment {
        out.push('#');
        out.push_str(fragment);
    }

    out
}

fn query_parameter_names(query: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut start = 0usize;

    loop {
        let end = query[start..]
            .find('&')
            .map(|offset| start + offset)
            .unwrap_or(query.len());
        let pair = &query[start..end];
        let separator = pair.find('=').unwrap_or(pair.len());
        let name = percent_decode_query_component(&pair[..separator], false);
        if !names.iter().any(|existing| existing == &name) {
            names.push(name);
        }

        if end == query.len() {
            break;
        }
        start = end + 1;
        if start >= query.len() {
            break;
        }
    }

    names
}

fn android_get_query_parameter(query: &str, wanted_name: &str) -> Option<String> {
    let encoded_name = uri_encode_component(wanted_name);
    let mut start = 0usize;

    loop {
        let end = query[start..]
            .find('&')
            .map(|offset| start + offset)
            .unwrap_or(query.len());
        let pair = &query[start..end];
        let separator = pair.find('=').unwrap_or(pair.len());
        if &pair[..separator] == encoded_name {
            if separator == pair.len() {
                return Some(String::new());
            }
            return Some(percent_decode_query_component(&pair[separator + 1..], true));
        }

        if end == query.len() {
            break;
        }
        start = end + 1;
        if start >= query.len() {
            break;
        }
    }

    None
}

fn parse_url(src: &str) -> Option<ParsedUrl<'_>> {
    let (scheme, rest) = src.split_once("://")?;
    let (without_fragment, fragment) = match rest.split_once('#') {
        Some((head, tail)) => (head, Some(tail)),
        None => (rest, None),
    };
    let (without_query, query) = match without_fragment.split_once('?') {
        Some((head, tail)) => (head, Some(tail)),
        None => (without_fragment, None),
    };

    let authority_end = without_query.find('/').unwrap_or(without_query.len());
    let authority = &without_query[..authority_end];
    let path = if authority_end < without_query.len() {
        &without_query[authority_end..]
    } else {
        ""
    };

    let authority = authority.rsplit_once('@').map_or(authority, |(_, tail)| tail);
    let (host, port) = split_host_port(authority)?;
    if host.is_empty() {
        return None;
    }

    Some(ParsedUrl { scheme, host, port, path, query, fragment })
}

fn split_host_port(authority: &str) -> Option<(&str, Option<u16>)> {
    if authority.starts_with('[') {
        let close = authority.find(']')?;
        let host = &authority[..=close];
        let remainder = &authority[close + 1..];
        if remainder.is_empty() {
            return Some((host, None));
        }
        let port = remainder.strip_prefix(':')?.parse::<u16>().ok()?;
        return Some((host, Some(port)));
    }

    if let Some((host, candidate_port)) = authority.rsplit_once(':') {
        if !host.contains(':') {
            if let Ok(port) = candidate_port.parse::<u16>() {
                return Some((host, Some(port)));
            }
        }
    }
    Some((authority, None))
}

fn android_builder_path(raw_path: &str) -> String {
    let decoded = percent_decode_query_component(raw_path, false);
    uri_encode_path(&decoded)
}

fn legacy_general_builder_path(raw_path: &str) -> String {
    let decoded = percent_decode_query_component(raw_path, false);
    let segment = decoded.replacen('/', "", 1);
    format!("/{segment}")
}

fn uri_encode_path(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        let allowed = byte.is_ascii_alphanumeric()
            || matches!(byte, b'_' | b'-' | b'!' | b'.' | b'~' | b'\'' | b'(' | b')' | b'*' | b'/');
        if allowed {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}

fn percent_decode_query_component(input: &str, convert_plus: bool) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' if convert_plus => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    out.push((hi << 4) | lo);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            value => {
                out.push(value);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn uri_encode_component(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        let allowed = byte.is_ascii_alphanumeric()
            || matches!(byte, b'_' | b'-' | b'!' | b'.' | b'~' | b'\'' | b'(' | b')' | b'*');
        if allowed {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::clean_url;

    #[test]
    fn keeps_non_http_urls_unchanged() {
        assert_eq!(clean_url("tg://resolve?domain=test", true, &[]), "tg://resolve?domain=test");
    }

    #[test]
    fn strip_all_queries_matches_legacy_last_question_mark_behavior() {
        assert_eq!(super::strip_all_queries("https://example.com/a?x=?y"), "https://example.com/a?x=");
        assert_eq!(super::strip_all_queries("https://example.com/a"), "https://example.com/a");
    }

    #[test]
    fn rewrites_twitter_preview_and_drops_query() {
        assert_eq!(
            clean_url("https://fxtwitter.com/user/status/1?utm_source=test#x", true, &[]),
            "https://twitter.com/user/status/1"
        );
    }

    #[test]
    fn preserves_preview_host_when_patch_disabled() {
        assert_eq!(
            clean_url("http://fixupx.com/u/status/2?foo=bar", false, &[]),
            "https://fixupx.com/u/status/2"
        );
    }

    #[test]
    fn youtube_keeps_only_timestamp() {
        assert_eq!(
            clean_url("https://youtube.com/watch?v=abc&t=90&utm_source=x", true, &[]),
            "https://youtube.com/watch?t=90"
        );
    }

    #[test]
    fn removes_tracking_and_custom_queries() {
        assert_eq!(
            clean_url("https://example.com/a?foo=1&utm_source=x&clid=2&secret=3&bar=4#frag", true, &["secret"]),
            "https://example.com/a?foo=1&bar=4#frag"
        );
    }

    #[test]
    fn general_builder_always_appends_a_path_segment() {
        assert_eq!(
            clean_url("https://example.com?x=1", true, &[]),
            "https://example.com/?x=1"
        );
    }

    #[test]
    fn general_builder_uses_decoded_path_as_encoded_path() {
        assert_eq!(
            clean_url("https://example.com/a%20b%2Fc?x=1", true, &[]),
            "https://example.com/a b/c?x=1"
        );
    }

    #[test]
    fn twitter_builder_decodes_then_reencodes_path() {
        assert_eq!(
            clean_url("https://twitter.com/a%2Fb+c", true, &[]),
            "https://twitter.com/a/b%2Bc"
        );
    }

    #[test]
    fn removes_default_port_but_keeps_custom_port() {
        assert_eq!(clean_url("https://example.com:443/a?x=1", true, &[]), "https://example.com/a?x=1");
        assert_eq!(clean_url("https://example.com:8443/a?x=1", true, &[]), "https://example.com:8443/a?x=1");
    }

    #[test]
    fn filters_percent_encoded_query_names() {
        assert_eq!(
            clean_url("https://example.com/?utm%5Fsource=x&ok=1", true, &[]),
            "https://example.com/?ok=1"
        );
    }

    #[test]
    fn canonicalizes_query_like_android_uri_builder() {
        assert_eq!(
            clean_url("https://example.com/?q=a+b&flag", true, &[]),
            "https://example.com/?q=a%20b&flag="
        );
    }

    #[test]
    fn youtube_timestamp_decodes_plus_like_android_uri() {
        assert_eq!(
            clean_url("https://youtube.com/watch?t=1+2", true, &[]),
            "https://youtube.com/watch?t=1%202"
        );
    }

    #[test]
    fn trailing_ampersand_does_not_add_empty_parameter() {
        assert_eq!(
            clean_url("https://example.com/?a=1&", true, &[]),
            "https://example.com/?a=1"
        );
    }

    #[test]
    fn noncanonical_query_key_matches_android_null_lookup_behavior() {
        assert_eq!(
            clean_url("https://example.com/?fo%6f=bar&ok=1", true, &[]),
            "https://example.com/?foo=null&ok=1"
        );
    }

    #[test]
    fn matches_android_first_value_behavior_for_duplicate_names() {
        assert_eq!(
            clean_url("https://example.com/?a=1&a=2&b=3", true, &[]),
            "https://example.com/?a=1&b=3"
        );
    }
}
