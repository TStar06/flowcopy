//! Tier-0 "smart formatting" for transcription output.
//!
//! Runs after filler-word filtering and before optional LLM post-processing.
//! Everything here is deterministic, offline and language-table driven:
//! - spoken commands ("neue Zeile", "Punkt", "new line", "period", ...)
//! - punctuation/whitespace normalization
//! - sentence-start capitalization (with an abbreviation guard)
//!
//! Implementation is token-based rather than regex-based: the `regex` crate
//! has no lookaround, and the guards against false positives (e.g. German
//! "der Punkt ist" vs. the spoken command "punkt") need to inspect the
//! surrounding tokens anyway.

/// What a spoken command expands to.
#[derive(Clone, Copy, PartialEq)]
enum CommandKind {
    /// Punctuation that attaches to the previous word ("gut punkt" -> "gut.")
    Punct(char),
    /// A line break replacing the token(s)
    NewLine,
    /// A paragraph break replacing the token(s)
    NewParagraph,
    /// Opening bracket: attaches to the FOLLOWING word
    OpenParen,
    /// Closing bracket: attaches to the previous word
    CloseParen,
}

/// A spoken command: a phrase of 1..=2 lowercase words and its expansion.
struct SpokenCommand {
    phrase: &'static [&'static str],
    kind: CommandKind,
    /// Commands like "punkt"/"period" collide with real nouns; they get the
    /// determiner/number guards. Unambiguous phrases ("neue zeile") do not.
    ambiguous: bool,
}

const DE_COMMANDS: &[SpokenCommand] = &[
    SpokenCommand {
        phrase: &["neue", "zeile"],
        kind: CommandKind::NewLine,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["nächste", "zeile"],
        kind: CommandKind::NewLine,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["neuer", "absatz"],
        kind: CommandKind::NewParagraph,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["neuen", "absatz"],
        kind: CommandKind::NewParagraph,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["punkt"],
        kind: CommandKind::Punct('.'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["komma"],
        kind: CommandKind::Punct(','),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["fragezeichen"],
        kind: CommandKind::Punct('?'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["ausrufezeichen"],
        kind: CommandKind::Punct('!'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["doppelpunkt"],
        kind: CommandKind::Punct(':'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["semikolon"],
        kind: CommandKind::Punct(';'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["klammer", "auf"],
        kind: CommandKind::OpenParen,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["klammer", "zu"],
        kind: CommandKind::CloseParen,
        ambiguous: false,
    },
];

const EN_COMMANDS: &[SpokenCommand] = &[
    SpokenCommand {
        phrase: &["new", "line"],
        kind: CommandKind::NewLine,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["new", "paragraph"],
        kind: CommandKind::NewParagraph,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["period"],
        kind: CommandKind::Punct('.'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["full", "stop"],
        kind: CommandKind::Punct('.'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["comma"],
        kind: CommandKind::Punct(','),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["question", "mark"],
        kind: CommandKind::Punct('?'),
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["exclamation", "mark"],
        kind: CommandKind::Punct('!'),
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["exclamation", "point"],
        kind: CommandKind::Punct('!'),
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["colon"],
        kind: CommandKind::Punct(':'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["semicolon"],
        kind: CommandKind::Punct(';'),
        ambiguous: true,
    },
    SpokenCommand {
        phrase: &["open", "paren"],
        kind: CommandKind::OpenParen,
        ambiguous: false,
    },
    SpokenCommand {
        phrase: &["close", "paren"],
        kind: CommandKind::CloseParen,
        ambiguous: false,
    },
];

/// Words that mark a preceding token as "this is a noun phrase, not a
/// command": determiners/articles directly before "Punkt"/"Komma" etc.
const DE_DETERMINERS: &[&str] = &[
    "der",
    "die",
    "das",
    "den",
    "dem",
    "des",
    "ein",
    "eine",
    "einen",
    "einem",
    "einer",
    "kein",
    "keine",
    "keinen",
    "keinem",
    "jeder",
    "jede",
    "jeden",
    "jedem",
    "dieser",
    "diese",
    "diesen",
    "diesem",
    "welcher",
    "welchen",
    "zum",
    "im",
    "vom",
    "beim",
    "am",
    "wichtige",
    "wichtigen",
    "wichtigsten",
    "letzte",
    "letzten",
    "erste",
    "ersten",
    "zweite",
    "zweiten",
    "dritte",
    "dritten",
    "entscheidende",
    "entscheidenden",
    "springende",
    "springenden",
    "um",
];

const EN_DETERMINERS: &[&str] = &[
    "a", "the", "every", "each", "one", "this", "that", "another", "no", "any", "some", "first",
    "second", "third", "last", "grace", "trial", "time",
];

/// German/English abbreviations after which a '.' does not end a sentence.
/// Compared against the lowercased word before the dot (dot excluded).
const ABBREVIATIONS: &[&str] = &[
    // German
    "z.b", "d.h", "u.a", "u.u", "z.t", "ca", "bzw", "evtl", "ggf", "inkl", "exkl", "max", "min",
    "mio", "mrd", "nr", "s", "str", "usw", "vgl", "dr", "prof", "abs", "bsp", "etc", "sog", "tel",
    // English
    "e.g", "i.e", "mr", "mrs", "ms", "vs", "approx", "dept", "est", "fig", "inc", "jr", "sr", "no",
    "st", "vol",
];

fn is_opaque(token: &str) -> bool {
    token.contains("://") || token.contains('@') || token.to_lowercase().starts_with("www.")
}

/// Strips leading/trailing punctuation for matching, returns lowercase core.
fn match_key(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn commands_for_language(lang: &str) -> Vec<&'static SpokenCommand> {
    let base = lang.split(&['-', '_'][..]).next().unwrap_or(lang);
    match base {
        "de" => DE_COMMANDS.iter().collect(),
        "en" => EN_COMMANDS.iter().collect(),
        // auto / anything else: both tables (command words are distinctive
        // enough across the two languages we target)
        _ => DE_COMMANDS.iter().chain(EN_COMMANDS.iter()).collect(),
    }
}

fn determiners_for_language(lang: &str) -> Vec<&'static str> {
    let base = lang.split(&['-', '_'][..]).next().unwrap_or(lang);
    match base {
        "de" => DE_DETERMINERS.to_vec(),
        "en" => EN_DETERMINERS.to_vec(),
        _ => DE_DETERMINERS
            .iter()
            .chain(EN_DETERMINERS.iter())
            .copied()
            .collect(),
    }
}

/// True when the token looks like a number ("5", "12.", "3,5") or a German/
/// English number word — used to veto "Punkt 5" / "um Punkt 12" style hits.
fn is_number_like(token: &str) -> bool {
    let key = match_key(token);
    if key
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        return true;
    }
    matches!(
        key.as_str(),
        "null"
            | "eins"
            | "zwei"
            | "drei"
            | "vier"
            | "fünf"
            | "sechs"
            | "sieben"
            | "acht"
            | "neun"
            | "zehn"
            | "elf"
            | "zwölf"
            | "one"
            | "two"
            | "three"
            | "four"
            | "five"
            | "six"
            | "seven"
            | "eight"
            | "nine"
            | "ten"
            | "eleven"
            | "twelve"
    )
}

/// Output element while assembling the formatted text.
enum Piece {
    Word(String),
    Break(&'static str),
}

/// Replaces spoken commands in a token stream and returns the piece list.
fn apply_spoken_commands(tokens: Vec<String>, lang: &str) -> Vec<Piece> {
    let commands = commands_for_language(lang);
    let determiners = determiners_for_language(lang);
    let mut pieces: Vec<Piece> = Vec::with_capacity(tokens.len());
    let mut i = 0;

    'outer: while i < tokens.len() {
        if !is_opaque(&tokens[i]) {
            for cmd in &commands {
                let n = cmd.phrase.len();
                if i + n > tokens.len() {
                    continue;
                }
                let window_matches = (0..n).all(|k| match_key(&tokens[i + k]) == cmd.phrase[k])
                    // A multi-word phrase must not have inner punctuation
                    // ("neue. Zeile" is not a command).
                    && (0..n.saturating_sub(1)).all(|k| {
                        !tokens[i + k].ends_with(|c: char| ".,!?;:".contains(c))
                    });
                if !window_matches {
                    continue;
                }

                if cmd.ambiguous {
                    // Guard 1: determiner directly before -> real noun.
                    if i > 0 {
                        let prev = match_key(&tokens[i - 1]);
                        if determiners.iter().any(|d| *d == prev) {
                            continue;
                        }
                    }
                    // Guard 2: number right after -> "Punkt 5", "Komma drei".
                    if i + n < tokens.len() && is_number_like(&tokens[i + n]) {
                        continue;
                    }
                    // Guard 3: command at the very beginning of the text is
                    // far more likely a real word ("Punkt eins:").
                    if i == 0 && pieces.is_empty() {
                        continue;
                    }
                }

                match cmd.kind {
                    CommandKind::Punct(p) => {
                        attach_punct(&mut pieces, p);
                        // The spoken command may carry the sentence's real
                        // punctuation from the ASR ("Punkt.") — drop it, the
                        // expansion IS the punctuation.
                    }
                    CommandKind::NewLine => {
                        strip_trailing_comma(&mut pieces);
                        pieces.push(Piece::Break("\n"));
                    }
                    CommandKind::NewParagraph => {
                        strip_trailing_comma(&mut pieces);
                        pieces.push(Piece::Break("\n\n"));
                    }
                    CommandKind::OpenParen => pieces.push(Piece::Word("(".to_string())),
                    CommandKind::CloseParen => attach_punct(&mut pieces, ')'),
                }
                i += n;
                continue 'outer;
            }
        }
        pieces.push(Piece::Word(tokens[i].clone()));
        i += 1;
    }
    pieces
}

/// Attaches a punctuation character to the last word piece, replacing any
/// trailing sentence punctuation so "gut. punkt" cannot become "gut..".
fn attach_punct(pieces: &mut Vec<Piece>, p: char) {
    for piece in pieces.iter_mut().rev() {
        if let Piece::Word(w) = piece {
            while w.ends_with(|c: char| ".,!?;:".contains(c)) && !w.ends_with("...") {
                w.pop();
            }
            if w.is_empty() {
                w.push(p);
            } else {
                w.push(p);
            }
            return;
        }
    }
    // Nothing before the command: emit the bare punctuation as its own word.
    pieces.push(Piece::Word(p.to_string()));
}

/// Removes a trailing comma before an inserted line break ("gut, neue zeile"
/// -> "gut\n", not "gut,\n").
fn strip_trailing_comma(pieces: &mut Vec<Piece>) {
    for piece in pieces.iter_mut().rev() {
        if let Piece::Word(w) = piece {
            if w.ends_with(',') {
                w.pop();
            }
            return;
        }
    }
}

/// Joins pieces into a string: words separated by single spaces, breaks
/// verbatim, opening parens glued to the following word.
fn join_pieces(pieces: Vec<Piece>) -> String {
    let mut out = String::new();
    let mut glue_next = false; // after "("
    for piece in pieces {
        match piece {
            Piece::Word(w) => {
                if w.is_empty() {
                    continue;
                }
                let is_open_paren = w == "(";
                if !out.is_empty() && !out.ends_with('\n') && !glue_next {
                    out.push(' ');
                }
                out.push_str(&w);
                glue_next = is_open_paren;
            }
            Piece::Break(b) => {
                out.push_str(b);
                glue_next = false;
            }
        }
    }
    out
}

/// Word-level punctuation cleanup that is safe without lookarounds:
/// collapses duplicate terminal punctuation (keeping "..." intact) and drops
/// stray leading commas/periods glued to words by earlier passes.
fn tidy_word(word: &str) -> String {
    if is_opaque(word) {
        return word.to_string();
    }
    let mut w = word.to_string();
    // ".." -> "." but keep real ellipses
    while w.ends_with("..") && !w.ends_with("...") {
        w.pop();
    }
    for p in [",,", "!!", "??", "::", ";;"] {
        while w.ends_with(p) {
            w.pop();
        }
    }
    w
}

/// True when the sentence may continue after this word despite it ending in
/// '.' — i.e. the word is a known abbreviation or a number like "3.".
fn ends_with_non_terminal_dot(word: &str) -> bool {
    let stem = word.trim_end_matches('.');
    if stem.chars().all(|c| c.is_ascii_digit()) && !stem.is_empty() {
        return true;
    }
    ABBREVIATIONS.contains(&stem.to_lowercase().as_str())
}

/// Capitalizes the first letter of a word unless it looks intentional
/// (opaque tokens, ALL-CAPS acronyms, camelCase identifiers).
fn capitalize(word: &str) -> String {
    if is_opaque(word) {
        return word.to_string();
    }
    let mut chars: Vec<char> = word.chars().collect();
    if let Some(first) = chars.iter().position(|c| c.is_alphabetic()) {
        // Leave camelCase/iPhone-style tokens alone
        let has_inner_upper = chars.iter().skip(first + 1).any(|c| c.is_uppercase());
        if !has_inner_upper {
            let upper: Vec<char> = chars[first].to_uppercase().collect();
            chars.splice(first..first + 1, upper);
        }
    }
    chars.into_iter().collect()
}

/// Applies sentence-start capitalization across the joined text.
fn capitalize_sentences(text: &str) -> String {
    let mut out_lines: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let mut words: Vec<String> = line.split(' ').map(|s| s.to_string()).collect();
        let mut capitalize_next = true; // every line starts a sentence
        for w in words.iter_mut() {
            if w.is_empty() {
                continue;
            }
            if capitalize_next {
                *w = capitalize(w);
            }
            capitalize_next = w.ends_with(['.', '!', '?'])
                && !w.ends_with("...")
                && !(w.ends_with('.') && ends_with_non_terminal_dot(w));
        }
        out_lines.push(words.join(" "));
    }
    out_lines.join("\n")
}

/// Entry point: applies spoken commands (optional) plus punctuation and
/// capitalization normalization to a filtered transcript.
///
/// `lang` is the *transcription* language setting ("auto", "de", "en-US", ...).
pub fn apply_smart_formatting(text: &str, lang: &str, spoken_commands: bool) -> String {
    if text.trim().is_empty() {
        return text.trim().to_string();
    }

    let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();

    let pieces = if spoken_commands {
        apply_spoken_commands(tokens, lang)
    } else {
        tokens.into_iter().map(Piece::Word).collect()
    };

    // Tidy each word piece, then join and capitalize.
    let pieces: Vec<Piece> = pieces
        .into_iter()
        .map(|p| match p {
            Piece::Word(w) => Piece::Word(tidy_word(&w)),
            b => b,
        })
        .collect();

    let joined = join_pieces(pieces);
    capitalize_sentences(&joined).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_german_period_and_newline() {
        let result =
            apply_smart_formatting("das ist gut punkt neue zeile nächster satz", "de", true);
        assert_eq!(result, "Das ist gut.\nNächster satz");
    }

    #[test]
    fn test_german_command_with_asr_punctuation() {
        // ASR often renders the command capitalized and with its own period
        let result =
            apply_smart_formatting("Das ist gut. Punkt. Neue Zeile. Weiter geht es", "de", true);
        assert_eq!(result, "Das ist gut.\nWeiter geht es");
    }

    #[test]
    fn test_german_noun_punkt_is_preserved() {
        let result = apply_smart_formatting("der Punkt ist wichtig", "de", true);
        assert_eq!(result, "Der Punkt ist wichtig");
    }

    #[test]
    fn test_german_punkt_before_number_is_preserved() {
        let result = apply_smart_formatting("wir treffen uns um Punkt 12", "de", true);
        assert_eq!(result, "Wir treffen uns um Punkt 12");
    }

    #[test]
    fn test_german_comma_command() {
        let result =
            apply_smart_formatting("hallo Tom komma wie geht es dir fragezeichen", "de", true);
        assert_eq!(result, "Hallo Tom, wie geht es dir?");
    }

    #[test]
    fn test_english_commands() {
        let result = apply_smart_formatting(
            "that looks great period new line see you tomorrow",
            "en",
            true,
        );
        assert_eq!(result, "That looks great.\nSee you tomorrow");
    }

    #[test]
    fn test_english_noun_period_is_preserved() {
        let result = apply_smart_formatting("the trial period ends tomorrow", "en", true);
        assert_eq!(result, "The trial period ends tomorrow");
    }

    #[test]
    fn test_commands_disabled() {
        let result = apply_smart_formatting("das ist gut punkt neue zeile weiter", "de", false);
        assert_eq!(result, "Das ist gut punkt neue zeile weiter");
    }

    #[test]
    fn test_capitalization_after_sentence() {
        let result = apply_smart_formatting("hallo. wie geht es dir? gut, danke", "de", true);
        assert_eq!(result, "Hallo. Wie geht es dir? Gut, danke");
    }

    #[test]
    fn test_abbreviation_does_not_capitalize() {
        let result = apply_smart_formatting("wir brauchen z.B. mehr Zeit", "de", true);
        assert_eq!(result, "Wir brauchen z.B. mehr Zeit");
    }

    #[test]
    fn test_duplicate_punctuation_collapsed() {
        let result = apply_smart_formatting("gut.. dann bis morgen", "de", true);
        assert_eq!(result, "Gut. Dann bis morgen");
    }

    #[test]
    fn test_ellipsis_preserved() {
        // An ellipsis marks a trailing thought, not a sentence end — the
        // continuation stays lowercase.
        let result = apply_smart_formatting("na ja... mal sehen", "de", true);
        assert_eq!(result, "Na ja... mal sehen");
    }

    #[test]
    fn test_urls_untouched() {
        let result =
            apply_smart_formatting("schau auf https://example.com/Test?a=1 nach", "de", true);
        assert_eq!(result, "Schau auf https://example.com/Test?a=1 nach");
    }

    #[test]
    fn test_email_untouched() {
        let result = apply_smart_formatting("schreib an Toni.Schmid@example.com bitte", "de", true);
        assert_eq!(result, "Schreib an Toni.Schmid@example.com bitte");
    }

    #[test]
    fn test_auto_language_applies_both_tables() {
        let de = apply_smart_formatting("alles klar punkt neue zeile weiter", "auto", true);
        assert_eq!(de, "Alles klar.\nWeiter");
        let en = apply_smart_formatting("sounds good period new line bye", "auto", true);
        assert_eq!(en, "Sounds good.\nBye");
    }

    #[test]
    fn test_paragraph_break() {
        let result = apply_smart_formatting("erster teil neuer absatz zweiter teil", "de", true);
        assert_eq!(result, "Erster teil\n\nZweiter teil");
    }

    #[test]
    fn test_trailing_comma_stripped_before_break() {
        let result = apply_smart_formatting("das ist gut, neue Zeile, weiter im Text", "de", true);
        assert_eq!(result, "Das ist gut\nWeiter im Text");
    }

    #[test]
    fn test_parens() {
        let result = apply_smart_formatting(
            "das Ergebnis klammer auf siehe Anhang klammer zu steht fest",
            "de",
            true,
        );
        assert_eq!(result, "Das Ergebnis (siehe Anhang) steht fest");
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(apply_smart_formatting("   ", "de", true), "");
    }

    #[test]
    fn test_question_mark_command_german() {
        let result = apply_smart_formatting("kommst du morgen fragezeichen", "de", true);
        assert_eq!(result, "Kommst du morgen?");
    }
}
