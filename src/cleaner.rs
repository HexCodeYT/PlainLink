use crate::rules::RuleSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanResult {
    pub original: String,
    pub cleaned: String,
    pub removed: Vec<RemovedParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedParam {
    pub name: String,
    pub value: Option<String>,
    pub reason: String,
}

impl CleanResult {
    pub fn changed(&self) -> bool {
        self.original.trim() != self.cleaned
    }
}

pub fn clean_url(input: &str, rules: &RuleSet) -> CleanResult {
    let candidate = input.trim();

    let Some(host) = extract_host(candidate) else {
        return unchanged(input);
    };

    let fragment_start = candidate.find('#');
    let query_start = candidate.find('?').filter(|query_index| {
        fragment_start.is_none_or(|fragment_index| *query_index < fragment_index)
    });

    let Some(query_start) = query_start else {
        return CleanResult {
            original: input.to_string(),
            cleaned: candidate.to_string(),
            removed: Vec::new(),
        };
    };

    let query_end = fragment_start.unwrap_or(candidate.len());
    let prefix = &candidate[..query_start];
    let query = &candidate[query_start + 1..query_end];
    let suffix = &candidate[query_end..];
    let mut kept_parts = Vec::new();
    let mut removed = Vec::new();

    for part in query.split('&') {
        if part.is_empty() {
            kept_parts.push(part);
            continue;
        }

        let (name, value) = split_query_part(part);

        if name.is_empty() {
            kept_parts.push(part);
            continue;
        }

        if let Some(reason) = rules.removal_reason(&host, name) {
            removed.push(RemovedParam {
                name: name.to_string(),
                value: value.map(str::to_string),
                reason,
            });
        } else {
            kept_parts.push(part);
        }
    }

    if removed.is_empty() {
        return CleanResult {
            original: input.to_string(),
            cleaned: candidate.to_string(),
            removed,
        };
    }

    let mut cleaned = String::from(prefix);

    if !kept_parts.is_empty() {
        cleaned.push('?');
        cleaned.push_str(&kept_parts.join("&"));
    }

    cleaned.push_str(suffix);

    CleanResult {
        original: input.to_string(),
        cleaned,
        removed,
    }
}

fn unchanged(input: &str) -> CleanResult {
    CleanResult {
        original: input.to_string(),
        cleaned: input.to_string(),
        removed: Vec::new(),
    }
}

fn split_query_part(part: &str) -> (&str, Option<&str>) {
    part.split_once('=')
        .map(|(name, value)| (name, Some(value)))
        .unwrap_or((part, None))
}

fn extract_host(url: &str) -> Option<String> {
    if url.is_empty() || url.chars().any(char::is_whitespace) {
        return None;
    }

    let (after_prefix, has_scheme) = if url
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        (&url[8..], true)
    } else if url
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
    {
        (&url[7..], true)
    } else if let Some(protocol_relative) = url.strip_prefix("//") {
        (protocol_relative, true)
    } else {
        if url.contains("://") || url.contains('@') {
            return None;
        }

        (url, false)
    };

    let authority_end = after_prefix
        .find(['/', '?', '#'])
        .unwrap_or(after_prefix.len());
    let authority = &after_prefix[..authority_end];

    if authority.is_empty() {
        return None;
    }

    let host_port = authority.rsplit('@').next().unwrap_or(authority);

    if host_port.starts_with('[') {
        let end = host_port.find(']')?;
        let host = &host_port[..=end];
        return (has_scheme && host.len() > 2).then(|| host.to_ascii_lowercase());
    }

    let host = host_port
        .split(':')
        .next()
        .unwrap_or(host_port)
        .trim_end_matches('.');

    (!host.is_empty() && (has_scheme || is_domain_like_host(host)))
        .then(|| host.to_ascii_lowercase())
}

fn is_domain_like_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    let mut labels = host.split('.');
    let Some(first) = labels.next() else {
        return false;
    };
    let remaining = labels.collect::<Vec<_>>();

    !remaining.is_empty() && valid_host_label(first) && remaining.into_iter().all(valid_host_label)
}

fn valid_host_label(label: &str) -> bool {
    let mut characters = label.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    let last = label.chars().last().unwrap_or(first);

    first.is_alphanumeric()
        && last.is_alphanumeric()
        && label
            .chars()
            .all(|character| character.is_alphanumeric() || character == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RuleSet;

    #[test]
    fn removes_youtube_short_share_token() {
        let rules = RuleSet::default_rules();
        let result = clean_url("https://youtu.be/LYa_ReqRlcs?si=VC4qVB_EUC90uwbo", &rules);

        assert_eq!(result.cleaned, "https://youtu.be/LYa_ReqRlcs");
        assert_eq!(result.removed[0].name, "si");
    }

    #[test]
    fn removes_tracking_from_schemeless_urls_without_adding_a_scheme() {
        let rules = RuleSet::default_rules();
        let result = clean_url("youtu.be/LYa_ReqRlcs?si=VC4qVB_EUC90uwbo", &rules);

        assert_eq!(result.cleaned, "youtu.be/LYa_ReqRlcs");
        assert_eq!(result.removed[0].name, "si");
    }

    #[test]
    fn removes_tracking_from_protocol_relative_urls() {
        let rules = RuleSet::default_rules();
        let result = clean_url("//example.com/read?id=42&utm_source=newsletter", &rules);

        assert_eq!(result.cleaned, "//example.com/read?id=42");
    }

    #[test]
    fn continues_to_support_schemeful_single_label_hosts() {
        let rules = RuleSet::default_rules();
        let result = clean_url("http://intranet/read?id=42&utm_source=newsletter", &rules);

        assert_eq!(result.cleaned, "http://intranet/read?id=42");
    }

    #[test]
    fn removes_global_trackers_and_preserves_fragment() {
        let rules = RuleSet::default_rules();
        let result = clean_url(
            "https://example.com/read?utm_source=newsletter&ok=1&fbclid=abc#comments",
            &rules,
        );

        assert_eq!(result.cleaned, "https://example.com/read?ok=1#comments");
        assert_eq!(result.removed.len(), 2);
    }

    #[test]
    fn keeps_unknown_parameters_by_default() {
        let rules = RuleSet::default_rules();
        let result = clean_url(
            "https://example.com/path?token=abc&utm_medium=email",
            &rules,
        );

        assert_eq!(result.cleaned, "https://example.com/path?token=abc");
    }

    #[test]
    fn preserves_required_youtube_parameters() {
        let rules = RuleSet::default_rules();
        let result = clean_url(
            "https://www.youtube.com/watch?v=abc123&si=tracking&list=playlist&t=12",
            &rules,
        );

        assert_eq!(
            result.cleaned,
            "https://www.youtube.com/watch?v=abc123&list=playlist&t=12"
        );
    }

    #[test]
    fn ignores_query_markers_inside_fragments() {
        let rules = RuleSet::default_rules();
        let input = "https://example.com/#/route?utm_source=not-a-query";

        assert_eq!(clean_url(input, &rules).cleaned, input);
    }

    #[test]
    fn leaves_non_urls_unchanged() {
        let rules = RuleSet::default_rules();
        let input = "copy this ordinary sentence";

        assert_eq!(clean_url(input, &rules).cleaned, input);
    }

    #[test]
    fn leaves_ambiguous_schemeless_text_unchanged() {
        let rules = RuleSet::default_rules();

        for input in [
            "readme/path?utm_source=notes",
            "person@example.com?utm_source=mail",
            "copy example.com/read?utm_source=notes",
            "ftp://example.com/file?utm_source=download",
        ] {
            assert_eq!(clean_url(input, &rules).cleaned, input);
        }
    }
}
