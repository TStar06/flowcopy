//! Deterministic e-mail layout for dictated text: a spoken greeting goes on
//! its own line, a trailing sign-off + name go on separate lines. This is the
//! guarantee the LLM tier can't give — it works offline, without an API key
//! and when the cloud call fails. Idempotent: already-formatted text (from
//! the LLM or a previous run) is rebuilt into the same canonical form.

use regex::Regex;
use std::sync::OnceLock;

/// Applies e-mail layout to `text`: trailing sign-off first (end-anchored),
/// then the greeting (start-anchored). Text without a spoken greeting or
/// sign-off is returned unchanged — nothing is ever invented.
pub fn apply_email_layout(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return text.to_string();
    }
    let with_signoff = layout_signoff(trimmed);
    layout_greeting(&with_signoff)
}

// ---------------------------------------------------------------------------
// Sign-off ("Mit freundlichen Grüßen Toni Schmidt" at the very end)
// ---------------------------------------------------------------------------

fn signoff_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // End-anchored: formula + 1-4 capitalized name tokens must close the
        // text, so the same phrase spoken mid-sentence never matches. The
        // capitalization requirement on name tokens rejects sentence
        // continuations ("liebe Grüße an das ganze Team").
        Regex::new(
            r"(?is)^(?P<body>.*?)(?P<pre>[,.!?;:]?)\s+(?P<formula>mit freundlichen gr(?:ü|u)(?:ß|ss)en|freundliche gr(?:ü|u)(?:ß|ss)e|viele gr(?:ü|u)(?:ß|ss)e|liebe gr(?:ü|u)(?:ß|ss)e|beste gr(?:ü|u)(?:ß|ss)e|sch(?:ö|o)ne gr(?:ü|u)(?:ß|ss)e|herzliche gr(?:ü|u)(?:ß|ss)e|danke und gru(?:ß|ss)|best regards|kind regards|warm regards|sincerely|cheers|lg|mfg|vg)\s*[,:]?\s+(?P<name>\p{Lu}[\p{L}\p{N}.'\-]*(?:[ \t]+\p{Lu}[\p{L}\p{N}.'\-]*){0,3})\s*\.?\s*$",
        )
        .expect("sign-off regex must compile")
    })
}

fn layout_signoff(text: &str) -> String {
    let caps = match signoff_re().captures(text) {
        Some(c) => c,
        None => return text.to_string(),
    };

    let body = caps.name("body").map_or("", |m| m.as_str()).trim_end();
    // A sign-off needs a message in front of it; a bare "Viele Grüße Toni"
    // dictation stays untouched.
    if body.is_empty() {
        return text.to_string();
    }

    let pre = caps.name("pre").map_or("", |m| m.as_str());
    let formula = caps.name("formula").map_or("", |m| m.as_str());
    // The name token class allows "." for abbreviations ("Dr."), so a
    // dictated sentence-final period ends up inside the capture — strip it.
    let name = caps
        .name("name")
        .map_or("", |m| m.as_str())
        .trim_end_matches('.');

    // Close the body as a sentence: keep real sentence punctuation, turn a
    // dictated comma/semicolon before the formula into a period.
    let body_closed = match pre {
        "." | "!" | "?" => format!("{body}{pre}"),
        _ => format!("{body}."),
    };

    format!("{}\n\n{}\n{}", body_closed, capitalize_first(formula), name)
}

// ---------------------------------------------------------------------------
// Greeting ("Hallo Frau Meier, …" / "Guten Tag, mein Name ist … ." at the start)
// ---------------------------------------------------------------------------

fn greeting_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^\s*(?P<greet>hallo|hello|hi|hey|moin|servus|guten[ \t]+(?:morgen|tag|abend)|sehr[ \t]+geehrte[rs]?|liebe[r]?|dear|good[ \t]+(?:morning|afternoon|evening))\b",
        )
        .expect("greeting regex must compile")
    })
}

fn self_intro_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(?:mein name ist|hier ist|ich bin|my name is|this is)\b")
            .expect("self-intro regex must compile")
    })
}

/// Longest prefix a greeting block may span. Longer means the ASR produced no
/// early punctuation and a break would be guesswork — better to leave it.
const GREETING_COMMA_LIMIT: usize = 60;
const GREETING_INTRO_LIMIT: usize = 120;

fn layout_greeting(text: &str) -> String {
    let m = match greeting_re().find(text) {
        Some(m) => m,
        None => return text.to_string(),
    };

    // "Liebe Grüße …" at the very start is a sign-off phrase, not a greeting
    // (the regex crate has no lookahead, so this guard lives here).
    let rest = &text[m.end()..];
    let greet_lower = m.as_str().trim().to_lowercase();
    if matches!(greet_lower.as_str(), "liebe" | "lieber") {
        let next_word = rest.split_whitespace().next().unwrap_or("");
        let next_lower = next_word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if matches!(next_lower.as_str(), "grüße" | "gruesse" | "grüsse") {
            return text.to_string();
        }
    }

    let comma_pos = rest.find(',').map(|p| m.end() + p);
    let sentence_pos = rest.find(['.', '!', '?']).map(|p| m.end() + p);

    let block_end = match (comma_pos, sentence_pos) {
        (None, None) => return text.to_string(),
        (Some(c), s) if s.map_or(true, |s| c < s) => {
            if c > GREETING_COMMA_LIMIT {
                return text.to_string();
            }
            // "Guten Tag, mein Name ist Toni Schmidt." — a self-introduction
            // after the comma extends the greeting block to the sentence end
            // (Wispr behavior: the whole intro forms one line).
            if self_intro_re().is_match(&text[c + 1..]) {
                match text[c + 1..].find(['.', '!', '?']) {
                    Some(p) if c + 1 + p <= GREETING_INTRO_LIMIT => c + 1 + p,
                    _ => return text.to_string(),
                }
            } else {
                c
            }
        }
        (_, Some(s)) => {
            if s > GREETING_COMMA_LIMIT {
                return text.to_string();
            }
            s
        }
        (Some(_), None) => unreachable!("covered by the guarded comma arm"),
    };

    let mut block = text[..=block_end].trim_end().to_string();
    let body = text[block_end + 1..].trim_start();
    // Greeting with no message after it — nothing to separate.
    if body.is_empty() {
        return text.to_string();
    }

    // Canonical greeting line ends with a comma ("Hallo Frau Meier,"), except
    // a self-introduction sentence, which keeps its period.
    if block.ends_with('.') && !self_intro_block(&block) {
        block.pop();
        block.push(',');
    }

    format!("{block}\n\n{body}")
}

fn self_intro_block(block: &str) -> bool {
    block
        .find(',')
        .map(|c| self_intro_re().is_match(&block[c + 1..]))
        .unwrap_or(false)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_email_greeting_body_signoff() {
        let result = apply_email_layout(
            "Hallo Frau Meier, ich schicke Ihnen die Unterlagen heute Nachmittag zu. Mit freundlichen Grüßen Toni Schmidt",
        );
        assert_eq!(
            result,
            "Hallo Frau Meier,\n\nich schicke Ihnen die Unterlagen heute Nachmittag zu.\n\nMit freundlichen Grüßen\nToni Schmidt"
        );
    }

    #[test]
    fn test_signoff_only() {
        let result =
            apply_email_layout("Bitte schickt mir die Unterlagen bis Freitag zu. Viele Grüße Toni");
        assert_eq!(
            result,
            "Bitte schickt mir die Unterlagen bis Freitag zu.\n\nViele Grüße\nToni"
        );
    }

    #[test]
    fn test_greeting_only() {
        let result = apply_email_layout("Hallo zusammen, das Meeting morgen fällt aus.");
        assert_eq!(result, "Hallo zusammen,\n\ndas Meeting morgen fällt aus.");
    }

    #[test]
    fn test_self_intro_extends_greeting_block() {
        let result = apply_email_layout(
            "Guten Tag, mein Name ist Toni Schmidt. Ich wollte fragen, ob der Bericht fertig ist. Mit freundlichen Grüßen, Toni Schmidt.",
        );
        assert_eq!(
            result,
            "Guten Tag, mein Name ist Toni Schmidt.\n\nIch wollte fragen, ob der Bericht fertig ist.\n\nMit freundlichen Grüßen\nToni Schmidt"
        );
    }

    #[test]
    fn test_multi_token_name() {
        let result = apply_email_layout(
            "Der Vertrag ist unterschrieben. Beste Grüße Dr. Anna Weber-Schmidt",
        );
        assert_eq!(
            result,
            "Der Vertrag ist unterschrieben.\n\nBeste Grüße\nDr. Anna Weber-Schmidt"
        );
    }

    #[test]
    fn test_english_variant() {
        let result = apply_email_layout(
            "Hi Sarah, please send the fleet report by Friday. Best regards John",
        );
        assert_eq!(
            result,
            "Hi Sarah,\n\nplease send the fleet report by Friday.\n\nBest regards\nJohn"
        );
    }

    #[test]
    fn test_signoff_with_asr_comma_and_period() {
        let result =
            apply_email_layout("Die Rechnung ist raus. Mit freundlichen Grüßen, Toni Schmidt.");
        assert_eq!(
            result,
            "Die Rechnung ist raus.\n\nMit freundlichen Grüßen\nToni Schmidt"
        );
    }

    #[test]
    fn test_no_greeting_no_signoff_is_untouched() {
        let input =
            "Der Termin morgen um 10 Uhr wird auf 14 Uhr verschoben. Bitte gebt das weiter.";
        assert_eq!(apply_email_layout(input), input);
    }

    #[test]
    fn test_idempotent_on_formatted_text() {
        let formatted = "Hallo Frau Meier,\n\nich schicke Ihnen die Unterlagen zu.\n\nMit freundlichen Grüßen\nToni Schmidt";
        assert_eq!(apply_email_layout(formatted), formatted);
    }

    #[test]
    fn test_signoff_phrase_mid_text_is_untouched() {
        let input = "Bei mit freundlichen Grüßen Toni Schmidt wird auch automatisch eine neue Zeile gemacht.";
        assert_eq!(apply_email_layout(input), input);
    }

    #[test]
    fn test_liebe_gruesse_at_start_is_not_a_greeting() {
        let input = "Liebe Grüße auch an das ganze Team bitte.";
        assert_eq!(apply_email_layout(input), input);
    }

    #[test]
    fn test_body_comma_before_signoff_becomes_period() {
        let result = apply_email_layout("Bitte schickt mir die Unterlagen zu, liebe Grüße Toni");
        assert_eq!(
            result,
            "Bitte schickt mir die Unterlagen zu.\n\nLiebe Grüße\nToni"
        );
    }

    #[test]
    fn test_greeting_sentence_period_becomes_comma() {
        let result = apply_email_layout("Hallo Frau Meier. Ich schicke Ihnen die Unterlagen zu.");
        assert_eq!(
            result,
            "Hallo Frau Meier,\n\nIch schicke Ihnen die Unterlagen zu."
        );
    }

    #[test]
    fn test_existing_newlines_from_snippets_are_tolerated() {
        let input = "Anbei die Daten:\nZeile eins\nZeile zwei. Viele Grüße Toni";
        let result = apply_email_layout(input);
        assert_eq!(
            result,
            "Anbei die Daten:\nZeile eins\nZeile zwei.\n\nViele Grüße\nToni"
        );
    }

    #[test]
    fn test_formal_greeting_with_signoff() {
        let result = apply_email_layout(
            "Sehr geehrte Damen und Herren, anbei die Unterlagen zur Prüfung. Mit freundlichen Grüßen Toni Schmidt",
        );
        assert_eq!(
            result,
            "Sehr geehrte Damen und Herren,\n\nanbei die Unterlagen zur Prüfung.\n\nMit freundlichen Grüßen\nToni Schmidt"
        );
    }

    #[test]
    fn test_bare_signoff_without_body_is_untouched() {
        let input = "Viele Grüße Toni";
        assert_eq!(apply_email_layout(input), input);
    }

    #[test]
    fn test_greeting_without_body_is_untouched() {
        let input = "Guten Tag, mein Name ist Toni Schmidt.";
        assert_eq!(apply_email_layout(input), input);
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(apply_email_layout(""), "");
    }
}
