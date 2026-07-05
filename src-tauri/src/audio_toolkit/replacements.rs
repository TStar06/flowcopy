//! Word/phrase replacements for transcripts: user-defined dictionary rules
//! ("vs code" -> "VS Code") and voice-triggered snippets ("meine Signatur"
//! -> a stored text block). Both share the same whole-word matching.

use regex::RegexBuilder;

/// A single replacement rule. `case_sensitive` only affects matching; the
/// replacement text is always inserted verbatim.
pub struct ReplacementRule<'a> {
    pub pattern: &'a str,
    pub replacement: &'a str,
    pub case_sensitive: bool,
}

/// Applies whole-word replacement rules to `text`, longest pattern first so
/// overlapping rules ("neu" / "neu starten") resolve deterministically.
pub fn apply_replacement_rules(text: &str, rules: &[ReplacementRule]) -> String {
    let mut sorted: Vec<&ReplacementRule> = rules
        .iter()
        .filter(|r| !r.pattern.trim().is_empty())
        .collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.pattern.len()));

    let mut result = text.to_string();
    for rule in sorted {
        let pattern = build_word_pattern(rule.pattern);
        let regex = match RegexBuilder::new(&pattern)
            .case_insensitive(!rule.case_sensitive)
            .build()
        {
            Ok(r) => r,
            Err(_) => continue, // defensive: escaped patterns should always compile
        };
        // $0 etc. in user text must not act as capture references
        let replacement = rule.replacement.replace('$', "$$");
        result = regex.replace_all(&result, replacement.as_str()).to_string();
    }
    result
}

/// Builds a whole-word regex for a literal pattern. `\b` only works against
/// word characters, so it is applied per edge: a pattern ending in "." (e.g.
/// "z.b.") gets no trailing boundary.
fn build_word_pattern(literal: &str) -> String {
    let escaped = regex::escape(literal.trim());
    let starts_word = literal
        .trim()
        .chars()
        .next()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
    let ends_word = literal
        .trim()
        .chars()
        .last()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
    format!(
        "{}{}{}",
        if starts_word { r"\b" } else { "" },
        escaped,
        if ends_word { r"\b" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule<'a>(p: &'a str, r: &'a str) -> ReplacementRule<'a> {
        ReplacementRule {
            pattern: p,
            replacement: r,
            case_sensitive: false,
        }
    }

    #[test]
    fn test_simple_replacement() {
        let rules = [rule("vs code", "VS Code")];
        assert_eq!(
            apply_replacement_rules("ich öffne vs code jetzt", &rules),
            "ich öffne VS Code jetzt"
        );
    }

    #[test]
    fn test_case_insensitive_by_default() {
        let rules = [rule("vs code", "VS Code")];
        assert_eq!(
            apply_replacement_rules("Vs Code läuft", &rules),
            "VS Code läuft"
        );
    }

    #[test]
    fn test_case_sensitive_rule() {
        let rules = [ReplacementRule {
            pattern: "IT",
            replacement: "Informationstechnik",
            case_sensitive: true,
        }];
        assert_eq!(
            apply_replacement_rules("IT und it sind verschieden", &rules),
            "Informationstechnik und it sind verschieden"
        );
    }

    #[test]
    fn test_whole_word_only() {
        let rules = [rule("art", "Kunst")];
        assert_eq!(
            apply_replacement_rules("die Startseite zeigt art", &rules),
            "die Startseite zeigt Kunst"
        );
    }

    #[test]
    fn test_longest_pattern_wins() {
        let rules = [rule("neu", "NEU"), rule("neu starten", "neustarten")];
        assert_eq!(
            apply_replacement_rules("bitte neu starten und neu laden", &rules),
            "bitte neustarten und NEU laden"
        );
    }

    #[test]
    fn test_umlaut_boundaries() {
        let rules = [rule("grüße", "Grüße")];
        assert_eq!(
            apply_replacement_rules("liebe grüße an alle", &rules),
            "liebe Grüße an alle"
        );
    }

    #[test]
    fn test_dollar_in_replacement_is_literal() {
        let rules = [rule("preis", "$5")];
        assert_eq!(
            apply_replacement_rules("der preis steht", &rules),
            "der $5 steht"
        );
    }

    #[test]
    fn test_multiword_snippet_expansion() {
        let rules = [rule("meine signatur", "Viele Grüße\nToni Schmid")];
        assert_eq!(
            apply_replacement_rules("das war's. Meine Signatur", &rules),
            "das war's. Viele Grüße\nToni Schmid"
        );
    }

    #[test]
    fn test_empty_pattern_ignored() {
        let rules = [rule("", "x")];
        assert_eq!(
            apply_replacement_rules("nichts passiert", &rules),
            "nichts passiert"
        );
    }

    #[test]
    fn test_trigger_with_trailing_punctuation() {
        let rules = [rule("meine adresse", "Musterweg 1, 12345 Beispielstadt")];
        assert_eq!(
            apply_replacement_rules("schick es an meine Adresse.", &rules),
            "schick es an Musterweg 1, 12345 Beispielstadt."
        );
    }
}
