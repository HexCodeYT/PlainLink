use std::fmt;

const DEFAULT_RULES: &str = include_str!("../rules/base.plainlink");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSet {
    pub global_remove: Vec<ParamPattern>,
    pub global_keep: Vec<ParamPattern>,
    pub domains: Vec<DomainRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainRule {
    pub domain: String,
    pub remove: Vec<ParamPattern>,
    pub keep: Vec<ParamPattern>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamPattern {
    Exact(String),
    Prefix(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    pub line: usize,
    pub message: String,
}

impl RuleSet {
    pub fn default_rules() -> Self {
        Self::parse(DEFAULT_RULES).expect("bundled PlainLink rules must parse")
    }

    pub fn parse(input: &str) -> Result<Self, RuleError> {
        let mut rules = RuleSet {
            global_remove: Vec::new(),
            global_keep: Vec::new(),
            domains: Vec::new(),
        };
        let mut current_domain: Option<usize> = None;

        for (index, raw_line) in input.lines().enumerate() {
            let line_number = index + 1;
            let line = strip_comment(raw_line).trim();

            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = line[1..line.len() - 1].trim();

                if section.eq_ignore_ascii_case("global") {
                    current_domain = None;
                    continue;
                }

                if let Some(domain) = section.strip_prefix("domain:") {
                    let normalized = normalize_domain(domain);
                    if normalized.is_empty() {
                        return Err(RuleError::new(line_number, "domain section is empty"));
                    }

                    rules.domains.push(DomainRule {
                        domain: normalized,
                        remove: Vec::new(),
                        keep: Vec::new(),
                    });
                    current_domain = Some(rules.domains.len() - 1);
                    continue;
                }

                return Err(RuleError::new(
                    line_number,
                    format!("unknown section [{section}]"),
                ));
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(RuleError::new(line_number, "expected key = value"));
            };

            let key = key.trim().to_ascii_lowercase();
            let patterns = parse_pattern_list(value);

            match (current_domain, key.as_str()) {
                (None, "remove") => rules.global_remove.extend(patterns),
                (None, "keep") => rules.global_keep.extend(patterns),
                (Some(domain_index), "remove") => {
                    rules.domains[domain_index].remove.extend(patterns);
                }
                (Some(domain_index), "keep") => {
                    rules.domains[domain_index].keep.extend(patterns);
                }
                (_, other) => {
                    return Err(RuleError::new(
                        line_number,
                        format!("unknown rule key `{other}`"),
                    ));
                }
            }
        }

        Ok(rules)
    }

    pub fn removal_reason(&self, host: &str, param_name: &str) -> Option<String> {
        let matching_domains = self.matching_domains(host);

        if self
            .global_keep
            .iter()
            .any(|pattern| pattern.matches(param_name))
            || matching_domains.iter().any(|domain| {
                domain
                    .keep
                    .iter()
                    .any(|pattern| pattern.matches(param_name))
            })
        {
            return None;
        }

        for domain in &matching_domains {
            if domain
                .remove
                .iter()
                .any(|pattern| pattern.matches(param_name))
            {
                return Some(format!("domain:{}", domain.domain));
            }
        }

        if self
            .global_remove
            .iter()
            .any(|pattern| pattern.matches(param_name))
        {
            return Some("global".to_string());
        }

        None
    }

    fn matching_domains(&self, host: &str) -> Vec<&DomainRule> {
        let host = normalize_domain(host);

        self.domains
            .iter()
            .filter(|rule| host == rule.domain || host.ends_with(&format!(".{}", rule.domain)))
            .collect()
    }
}

impl ParamPattern {
    pub fn parse(input: &str) -> Option<Self> {
        let value = input.trim().to_ascii_lowercase();

        if value.is_empty() {
            return None;
        }

        if let Some(prefix) = value.strip_suffix('*') {
            return Some(ParamPattern::Prefix(prefix.to_string()));
        }

        Some(ParamPattern::Exact(value))
    }

    pub fn matches(&self, param_name: &str) -> bool {
        let param_name = param_name.to_ascii_lowercase();

        match self {
            ParamPattern::Exact(expected) => param_name == *expected,
            ParamPattern::Prefix(prefix) => param_name.starts_with(prefix),
        }
    }
}

impl RuleError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for RuleError {}

impl fmt::Display for ParamPattern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamPattern::Exact(value) => formatter.write_str(value),
            ParamPattern::Prefix(prefix) => write!(formatter, "{prefix}*"),
        }
    }
}

fn parse_pattern_list(value: &str) -> Vec<ParamPattern> {
    value.split(',').filter_map(ParamPattern::parse).collect()
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(before_comment, _)| before_comment)
        .unwrap_or(line)
}

fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_and_domain_rules() {
        let rules = RuleSet::parse(
            r#"
            [global]
            remove = utm_*, fbclid

            [domain:youtube.com]
            remove = si
            keep = v, list
            "#,
        )
        .unwrap();

        assert_eq!(rules.global_remove.len(), 2);
        assert_eq!(rules.domains.len(), 1);
        assert_eq!(rules.domains[0].domain, "youtube.com");
    }

    #[test]
    fn matches_parent_domain_rules_against_subdomains() {
        let rules = RuleSet::parse(
            r#"
            [domain:youtube.com]
            remove = si
            "#,
        )
        .unwrap();

        assert_eq!(
            rules.removal_reason("www.youtube.com", "si"),
            Some("domain:youtube.com".to_string())
        );
    }

    #[test]
    fn keep_rules_win_over_remove_rules() {
        let rules = RuleSet::parse(
            r#"
            [global]
            remove = ref

            [domain:example.com]
            keep = ref
            "#,
        )
        .unwrap();

        assert_eq!(rules.removal_reason("example.com", "ref"), None);
    }
}
