//! ICU MessageFormat scaffold — `{placeholder}` substitution + the
//! `{count, plural, one {...} other {...}}` plural form. M8 does not
//! aim to be a full ICU implementation; the launch-keys baseline only
//! needs simple substitution + plural for "1 enemy spotted" / "N enemies
//! spotted" style strings.

/// Resolved plural form for a numeric count.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PluralForm {
    /// `count == 1`.
    One,
    /// All other counts.
    Other,
}

impl PluralForm {
    /// Resolve a plural form from a numeric count.
    pub fn from_count(count: i64) -> PluralForm {
        if count == 1 {
            PluralForm::One
        } else {
            PluralForm::Other
        }
    }
}

/// Substitute `{placeholder}` tokens in `template` with the supplied
/// (key, value) pairs. Plural-form ICU patterns are detected ahead of
/// substitution so the `#` and `{count}` literals inside the plural body
/// are honored.
pub fn format_with_args(template: &str, args: &[(&str, &str)]) -> String {
    if let Some(plural) = parse_plural_form(template, args) {
        return plural;
    }
    let mut out = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '{' {
            if let Some(end) = template[i + 1..].find('}') {
                let key = &template[i + 1..i + 1 + end];
                if !key.contains(',') {
                    if let Some((_, v)) = args.iter().find(|(k, _)| *k == key) {
                        out.push_str(v);
                        for _ in 0..end + 1 {
                            chars.next();
                        }
                        continue;
                    }
                }
            }
        }
        out.push(c);
    }
    out
}

/// Detect + apply the ICU `{count, plural, one {...} other {...}}` form.
/// Returns `Some(rendered)` when the template matches the plural shape
/// and an `args` entry named `count` resolves to a base-10 integer; falls
/// back to `None` (caller should run normal substitution) when the
/// pattern doesn't match.
pub fn parse_plural_form(template: &str, args: &[(&str, &str)]) -> Option<String> {
    let after_open = template.find("{count, plural,")?;
    let suffix = &template[after_open + "{count, plural,".len()..];
    let one_marker = suffix.find("one {")?;
    let one_start = one_marker + "one {".len();
    let one_end = find_matching_brace(&suffix[one_start..])?;
    let one_body = &suffix[one_start..one_start + one_end];

    let after_one = &suffix[one_start + one_end + 1..];
    let other_marker = after_one.find("other {")?;
    let other_start = other_marker + "other {".len();
    let other_end = find_matching_brace(&after_one[other_start..])?;
    let other_body = &after_one[other_start..other_start + other_end];

    let count_value = args.iter().find(|(k, _)| *k == "count").and_then(|(_, v)| v.parse::<i64>().ok())?;
    let chosen = match PluralForm::from_count(count_value) {
        PluralForm::One => one_body,
        PluralForm::Other => other_body,
    };
    let count_str = count_value.to_string();
    let mut substituted_args = Vec::with_capacity(args.len());
    substituted_args.extend(args.iter().copied());
    let rendered = format_with_args(chosen, &substituted_args);
    Some(rendered.replace('#', &count_str))
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth: i32 = 1;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Compose a plural-form template directly. Convenience for the localization
/// table consumers that need to construct a plural inline (e.g. dynamic
/// labels not present in the baseline en.json).
pub fn format_plural(count: i64, one: &str, other: &str) -> String {
    let template = format!("{{count, plural, one {{{one}}} other {{{other}}}}}");
    let count_str = count.to_string();
    format_with_args(&template, &[("count", count_str.as_str())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_simple_placeholder() {
        let s = format_with_args("hello {name}", &[("name", "world")]);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn unknown_placeholder_left_intact() {
        let s = format_with_args("hello {name}", &[]);
        assert_eq!(s, "hello {name}");
    }

    #[test]
    fn plural_one_branch() {
        let s = format_plural(1, "1 enemy spotted", "# enemies spotted");
        assert_eq!(s, "1 enemy spotted");
    }

    #[test]
    fn plural_other_branch() {
        let s = format_plural(3, "1 enemy spotted", "# enemies spotted");
        assert_eq!(s, "3 enemies spotted");
    }

    #[test]
    fn plural_zero_uses_other_branch() {
        let s = format_plural(0, "1 enemy spotted", "# enemies spotted");
        assert_eq!(s, "0 enemies spotted");
    }

    #[test]
    fn template_inline_plural_renders() {
        let template = "{count, plural, one {1 reload remaining} other {# reloads remaining}}";
        let s = format_with_args(template, &[("count", "5")]);
        assert_eq!(s, "5 reloads remaining");
    }
}
