//! Safety net for LLM dictation cleanup: verifies that the model output is a
//! permissible transformation of the transcript (filler removal, applied
//! self-corrections, punctuation/layout, number words to digits) and rejects
//! paraphrases, hallucinated content, and inflated rewrites. On rejection the
//! caller keeps the rule-based (Tier 0) text.

use std::collections::HashSet;

/// Prompt IDs whose output must stay verbatim (cleanup-only prompts).
/// Tone prompts (default_tone_formal/casual) and user-created prompts are
/// allowed to rephrase and are never checked.
pub const VERBATIM_PROMPT_IDS: &[&str] = &[
    "default_dictation_cleanup",
    "default_improve_transcriptions",
];

pub fn is_verbatim_prompt(prompt_id: &str) -> bool {
    VERBATIM_PROMPT_IDS.contains(&prompt_id)
}

/// Number words (folded form, see `fold`) mapped to their numeric value.
/// Covers German and English 0-19, tens, hundred, thousand plus the
/// inflected forms of German "ein".
const NUMBER_WORDS: &[(&str, u32)] = &[
    // German
    ("null", 0),
    ("ein", 1),
    ("eins", 1),
    ("eine", 1),
    ("einen", 1),
    ("einem", 1),
    ("einer", 1),
    ("zwei", 2),
    ("drei", 3),
    ("vier", 4),
    ("fuenf", 5),
    ("sechs", 6),
    ("sieben", 7),
    ("acht", 8),
    ("neun", 9),
    ("zehn", 10),
    ("elf", 11),
    ("zwoelf", 12),
    ("dreizehn", 13),
    ("vierzehn", 14),
    ("fuenfzehn", 15),
    ("sechzehn", 16),
    ("siebzehn", 17),
    ("achtzehn", 18),
    ("neunzehn", 19),
    ("zwanzig", 20),
    ("dreissig", 30),
    ("vierzig", 40),
    ("fuenfzig", 50),
    ("sechzig", 60),
    ("siebzig", 70),
    ("achtzig", 80),
    ("neunzig", 90),
    ("hundert", 100),
    ("tausend", 1000),
    // English
    ("zero", 0),
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
    ("seventeen", 17),
    ("eighteen", 18),
    ("nineteen", 19),
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
    ("hundred", 100),
    ("thousand", 1000),
];

/// German unit words as used inside compounds ("vierundzwanzig").
const GERMAN_COMPOUND_UNITS: &[(&str, u32)] = &[
    ("ein", 1),
    ("zwei", 2),
    ("drei", 3),
    ("vier", 4),
    ("fuenf", 5),
    ("sechs", 6),
    ("sieben", 7),
    ("acht", 8),
    ("neun", 9),
];

const GERMAN_TENS: &[(&str, u32)] = &[
    ("zwanzig", 20),
    ("dreissig", 30),
    ("vierzig", 40),
    ("fuenfzig", 50),
    ("sechzig", 60),
    ("siebzig", 70),
    ("achtzig", 80),
    ("neunzig", 90),
];

const ENGLISH_TENS: &[(&str, u32)] = &[
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
];

const ENGLISH_UNITS: &[(&str, u32)] = &[
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
];

/// Lowercase + fold German umlauts so that spelling variants the LLM may
/// produce ("Grüße" vs "Gruesse") compare equal.
fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            _ => out.push(c),
        }
    }
    out
}

/// Split folded text into runs of alphanumeric characters. Punctuation,
/// hyphens, `%`, `:` etc. are separators, so "14:30" -> ["14", "30"].
fn tokenize(text: &str) -> Vec<String> {
    let folded = fold(text);
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in folded.chars() {
        if c.is_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn number_value(token: &str) -> Option<u32> {
    NUMBER_WORDS
        .iter()
        .find(|(word, _)| *word == token)
        .map(|(_, value)| *value)
}

/// "vierundzwanzig" -> 24. Only the plain `<unit>und<tens>` shape.
fn german_compound_value(token: &str) -> Option<u32> {
    for (unit_word, unit) in GERMAN_COMPOUND_UNITS {
        let Some(rest) = token.strip_prefix(unit_word) else {
            continue;
        };
        let Some(tens_word) = rest.strip_prefix("und") else {
            continue;
        };
        if let Some((_, tens)) = GERMAN_TENS.iter().find(|(word, _)| *word == tens_word) {
            return Some(unit + tens);
        }
    }
    None
}

/// Digit strings the output may legitimately contain: digits already present
/// in the input plus values of spoken number words (incl. German compounds
/// and adjacent English tens+unit pairs like "twenty five").
fn allowed_digits(tokens: &[String]) -> HashSet<String> {
    let mut set = HashSet::new();
    for (i, token) in tokens.iter().enumerate() {
        if token.chars().all(|c| c.is_ascii_digit()) {
            set.insert(token.clone());
        }
        if let Some(value) = number_value(token) {
            set.insert(value.to_string());
        }
        if let Some(value) = german_compound_value(token) {
            set.insert(value.to_string());
        }
        if let Some((_, tens)) = ENGLISH_TENS
            .iter()
            .find(|(word, _)| *word == token.as_str())
        {
            if let Some(next) = tokens.get(i + 1) {
                if let Some((_, unit)) = ENGLISH_UNITS
                    .iter()
                    .find(|(word, _)| *word == next.as_str())
                {
                    set.insert((tens + unit).to_string());
                }
            }
        }
    }
    set
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{30FF}'   // Hiragana + Katakana
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{4E00}'..='\u{9FFF}' // CJK Unified Ideographs
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
    )
}

/// CJK text has no word boundaries, so token comparison is unreliable there.
fn is_mostly_cjk(text: &str) -> bool {
    let mut total = 0usize;
    let mut cjk = 0usize;
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        }
        total += 1;
        if is_cjk(c) {
            cjk += 1;
        }
    }
    total > 0 && cjk * 5 > total // > 20%
}

/// Ok(()) if `output` is a permissible cleanup of `input`, Err(reason) if the
/// LLM answer should be discarded in favor of the rule-based text.
pub fn check_verbatim(input: &str, output: &str) -> Result<(), String> {
    if is_mostly_cjk(input) {
        return Ok(());
    }

    let in_tokens = tokenize(input);
    let out_tokens = tokenize(output);

    if out_tokens.is_empty() {
        return Err("empty output".to_string());
    }

    // Every permitted operation removes words (fillers, discarded
    // self-corrections) or keeps the count (layout, punctuation); number word
    // conversion even shrinks it. Growth beyond a small slack means the model
    // added text.
    let in_len = in_tokens.len();
    let out_len = out_tokens.len();
    let max_len = in_len + std::cmp::max(3, in_len / 10);
    if out_len > max_len {
        return Err(format!(
            "inflated output ({} tokens from {} input tokens)",
            out_len, in_len
        ));
    }

    let in_set: HashSet<&str> = in_tokens.iter().map(|t| t.as_str()).collect();
    let allowed = allowed_digits(&in_tokens);
    let input_has_number_word = in_tokens
        .iter()
        .any(|t| number_value(t).is_some() || german_compound_value(t).is_some());

    let mut novel = 0usize;
    for token in &out_tokens {
        if in_set.contains(token.as_str()) {
            continue;
        }
        if token.chars().all(|c| c.is_ascii_digit()) {
            if allowed.contains(token) {
                continue;
            }
            // Compounds outside the table ("einhundertfuenf", ordinals):
            // accept short digit tokens as long as a number word was spoken.
            if token.len() <= 4 && input_has_number_word {
                continue;
            }
        }
        novel += 1;
    }

    // One novel token is tolerated (e.g. a fixed ASR typo); a paraphrase or a
    // hallucinated greeting immediately exceeds these bounds.
    if (novel >= 2 && novel * 10 > out_len) || novel >= 5 {
        return Err(format!(
            "{} novel tokens in {} output tokens",
            novel, out_len
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_passes() {
        let text = "Der Termin morgen um 10 Uhr wird verschoben.";
        assert!(check_verbatim(text, text).is_ok());
    }

    #[test]
    fn filler_removal_passes() {
        assert!(check_verbatim(
            "ähm der Termin äh morgen ist halt abgesagt",
            "Der Termin morgen ist abgesagt."
        )
        .is_ok());
    }

    #[test]
    fn number_word_de_passes() {
        assert!(check_verbatim(
            "der termin um vierzehn uhr dreißig",
            "Der Termin um 14:30 Uhr."
        )
        .is_ok());
    }

    #[test]
    fn number_word_percent_passes() {
        assert!(check_verbatim("zehn prozent rabatt", "10% Rabatt").is_ok());
    }

    #[test]
    fn number_compound_de_passes() {
        assert!(check_verbatim("vierundzwanzig stück bitte", "24 Stück bitte").is_ok());
    }

    #[test]
    fn number_word_en_passes() {
        assert!(check_verbatim("twenty five dollars for the ticket", "$25 for the ticket").is_ok());
    }

    #[test]
    fn self_correction_passes() {
        assert!(check_verbatim(
            "treffen wir uns um drei nein warte um vier",
            "Treffen wir uns um 4."
        )
        .is_ok());
    }

    #[test]
    fn bullet_markers_removed_passes() {
        assert!(check_verbatim(
            "wir brauchen drei sachen erster stichpunkt die angebote prüfen zweiter stichpunkt den termin bestätigen",
            "Wir brauchen drei Sachen:\n- die Angebote prüfen\n- den Termin bestätigen"
        )
        .is_ok());
    }

    #[test]
    fn self_correction_with_marker_passes() {
        assert!(check_verbatim(
            "ich rufe den elektriker an nee ich meine den klempner",
            "Ich rufe den Klempner an."
        )
        .is_ok());
    }

    #[test]
    fn restated_value_passes() {
        assert!(check_verbatim("das kostet zehn nein zwanzig euro", "Das kostet 20 Euro.").is_ok());
    }

    #[test]
    fn umlaut_variant_passes() {
        assert!(check_verbatim("viele grüße", "Viele Gruesse").is_ok());
    }

    #[test]
    fn layout_only_passes() {
        assert!(check_verbatim(
            "hallo team das meeting fällt aus viele grüße toni",
            "Hallo Team,\n\ndas Meeting fällt aus.\n\nViele Grüße\nToni"
        )
        .is_ok());
    }

    #[test]
    fn list_formatting_passes() {
        assert!(check_verbatim(
            "erstens den Bericht schicken zweitens den Termin bestätigen",
            "- den Bericht schicken\n- den Termin bestätigen"
        )
        .is_ok());
    }

    #[test]
    fn single_novel_word_tolerated() {
        assert!(check_verbatim(
            "wir sollten den standart prozess für alle neuen kunden ab sofort wieder verwenden",
            "Wir sollten den Standard-Prozess für alle neuen Kunden ab sofort wieder verwenden."
        )
        .is_ok());
    }

    #[test]
    fn paraphrase_rejected() {
        assert!(check_verbatim(
            "ich wollte fragen ob du morgen zeit hast",
            "Ich möchte mich erkundigen, ob du morgen verfügbar bist."
        )
        .is_err());
    }

    #[test]
    fn hallucinated_greeting_rejected() {
        assert!(check_verbatim(
            "die rechnung ist raus",
            "Hallo Herr Weber,\n\ndie Rechnung ist raus.\n\nMit freundlichen Grüßen\nAnna"
        )
        .is_err());
    }

    #[test]
    fn sentence_inflation_rejected() {
        assert!(check_verbatim(
            "der bericht ist fertig und liegt im ordner",
            "Der Bericht ist fertig. Außerdem liegt der Bericht im Ordner. Er wurde dort gespeichert."
        )
        .is_err());
    }

    #[test]
    fn empty_output_rejected() {
        assert!(check_verbatim("irgendwas wurde gesagt", "").is_err());
        assert!(check_verbatim("irgendwas wurde gesagt", "   \n ").is_err());
    }

    #[test]
    fn cjk_input_skipped() {
        assert!(check_verbatim("我想问一下你明天有没有时间", "明天你有空吗？").is_ok());
    }

    #[test]
    fn verbatim_prompt_ids() {
        assert!(is_verbatim_prompt("default_dictation_cleanup"));
        assert!(is_verbatim_prompt("default_improve_transcriptions"));
        assert!(!is_verbatim_prompt("default_tone_formal"));
        assert!(!is_verbatim_prompt("default_tone_casual"));
        assert!(!is_verbatim_prompt("my_custom_prompt"));
    }
}
