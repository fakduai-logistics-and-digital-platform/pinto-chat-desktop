//! Deep-link URI parser for `pinto://` links.
//!
//! Adapted from Telegram Desktop's `tg://resolve?domain=...` / `tg://settings/...`
//! URI routing, paired with `single_instance` handoff.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeepLink {
    OpenChat(String),
}

/// Parse a `pinto://` URI from CLI args or secondary-instance activation.
pub fn parse(raw: &str) -> Option<DeepLink> {
    let trimmed = raw.trim();
    let rest = trimmed.strip_prefix("pinto://")?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let path = path.trim_matches('/');

    if let Some(chat_id) = path.strip_prefix("chat/") {
        let id = chat_id.trim();
        return (!id.is_empty()).then(|| DeepLink::OpenChat(id.to_string()));
    }

    if path == "resolve" {
        for pair in query.split('&') {
            if let Some(("chat", val)) = pair.split_once('=') {
                let id = val.trim();
                if !id.is_empty() {
                    return Some(DeepLink::OpenChat(id.to_string()));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_and_resolve_links() {
        assert_eq!(
            parse("pinto://chat/room-42"),
            Some(DeepLink::OpenChat("room-42".into()))
        );
        assert_eq!(
            parse("pinto://resolve?foo=1&chat=abc-99"),
            Some(DeepLink::OpenChat("abc-99".into()))
        );
        assert_eq!(
            parse("  pinto://chat/room-42/  "),
            Some(DeepLink::OpenChat("room-42".into()))
        );
        assert_eq!(parse("pinto://resolve?chat="), None);
        assert_eq!(parse("pinto://chat/"), None);
        assert_eq!(parse("https://example.com"), None);
    }
}
