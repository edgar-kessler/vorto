//! The user's dictionary: replacements applied to every dictation, the hint that helps Whisper
//! spell names, the terms Parakeet checks against the audio, words from History worth adding,
//! and what Vorto learns from the corrections the user makes.
use crate::data::{Entry, Learned, Replacement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Applies every replacement, case-insensitively and only to whole words. `\n` in a
/// replacement becomes a line break, so "new paragraph" can turn into one.
pub fn replace(text: &str, replacements: &[Replacement]) -> String {
    let mut text = text.to_string();
    for r in replacements {
        let from: Vec<char> = r.from.trim().chars().collect();
        if from.is_empty() {
            continue;
        }
        let to = r.to.replace("\\n", "\n");
        text = replace_one(&text, &from, &to);
    }
    // A replacement that ends a line leaves the space that followed it at the start of the next.
    if text.contains('\n') {
        text = text
            .split('\n')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n");
    }
    text
}

fn same(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}
fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn replace_one(text: &str, from: &[char], to: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let fits = i + from.len() <= chars.len()
            && chars[i..i + from.len()]
                .iter()
                .zip(from)
                .all(|(&a, &b)| same(a, b));
        // Whole words only, unless the pattern itself starts or ends with punctuation.
        let starts = !word_char(from[0]) || i == 0 || !word_char(chars[i - 1]);
        let end = i + from.len();
        let ends = !word_char(from[from.len() - 1]) || end >= chars.len() || !word_char(chars[end]);
        if fits && starts && ends {
            out.push_str(to);
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// A short hint for Whisper, which spells words from it the same way.
pub fn whisper_prompt(vocabulary: &[String]) -> String {
    let mut prompt = String::new();
    for word in vocabulary {
        // Whisper takes the prompt as a C string: no NUL or other control characters.
        let word: String = word.chars().filter(|c| !c.is_control()).collect();
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        if prompt.len() + word.len() > 600 {
            break;
        }
        if !prompt.is_empty() {
            prompt.push_str(", ");
        }
        prompt.push_str(word);
    }
    if !prompt.is_empty() {
        prompt.push('.');
    }
    prompt
}

/// Words from History that look like names or terms: mixed case such as "GitHub", acronyms,
/// letters with digits, or words capitalized mid-sentence again and again. Most frequent first.
pub fn suggestions(history: &[Entry], known: &[String], dismissed: &[String]) -> Vec<String> {
    let mut seen: HashMap<String, (String, usize, bool)> = HashMap::new();
    for entry in history {
        let mut found: HashMap<String, (String, bool)> = HashMap::new();
        let mut sentence_start = true;
        for raw in entry.text.split_whitespace() {
            let word = raw.trim_matches(|c: char| !c.is_alphanumeric());
            let starts = sentence_start;
            sentence_start = raw.ends_with(['.', '!', '?', ':']);
            if word.chars().count() < 3 || word.chars().all(|c| c.is_numeric()) {
                continue;
            }
            let mut chars = word.chars();
            let first = chars.next().unwrap_or(' ');
            let rest: Vec<char> = chars.collect();
            let letters = word.chars().any(char::is_alphabetic);
            let shaped = (rest.iter().any(|c| c.is_uppercase())
                && rest.iter().any(|c| c.is_lowercase()))
                || (word
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .all(char::is_uppercase)
                    && word.chars().filter(|c| c.is_alphabetic()).count() >= 2)
                || (letters && word.chars().any(|c| c.is_ascii_digit()));
            let capitalized = !starts && first.is_uppercase() && !shaped;
            if shaped || capitalized {
                found
                    .entry(word.to_lowercase())
                    .or_insert((word.to_string(), shaped));
            }
        }
        for (key, (word, shaped)) in found {
            let slot = seen.entry(key).or_insert((word, 0, shaped));
            slot.1 += 1;
        }
    }
    let skip = |w: &str| {
        known
            .iter()
            .chain(dismissed)
            .any(|k| k.eq_ignore_ascii_case(w))
    };
    let mut list: Vec<(String, usize)> = seen
        .into_values()
        .filter(|(word, count, shaped)| *count >= if *shaped { 2 } else { 4 } && !skip(word))
        .map(|(word, count, _)| (word, count))
        .collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list.into_iter().take(12).map(|(w, _)| w).collect()
}

/// A word the user wants spelled their way, and how Vorto has misheard it before.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Term {
    pub term: String,
    pub sounds_like: Vec<String>,
}

/// Terms to check against the audio: what the user corrected most recently first, then the
/// dictionary. The engine stops at `MAX_TERMS`, so the words that matter most come first.
pub fn terms(vocabulary: &[String], learned: &[Learned]) -> Vec<Term> {
    let mut out: Vec<Term> = Vec::new();
    for l in learned.iter().rev() {
        match out.iter_mut().find(|t| t.term.eq_ignore_ascii_case(&l.to)) {
            Some(term) => term.sounds_like.push(l.from.clone()),
            None => out.push(Term {
                term: l.to.clone(),
                sounds_like: vec![l.from.clone()],
            }),
        }
    }
    for word in vocabulary {
        if !out.iter().any(|t| t.term.eq_ignore_ascii_case(word)) {
            out.push(Term {
                term: word.clone(),
                sounds_like: Vec::new(),
            });
        }
    }
    out.truncate(MAX_TERMS);
    out
}
pub const MAX_TERMS: usize = 64;

/// The dictionary for Whisper's hint and AI editing, with what the user corrected first.
pub fn words(vocabulary: &[String], learned: &[Learned]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let corrected = learned.iter().rev().map(|l| &l.to);
    for word in corrected.chain(vocabulary) {
        if !out.iter().any(|w| w.eq_ignore_ascii_case(word)) {
            out.push(word.clone());
        }
    }
    out
}

/// Swaps in what the user corrected, for spellings no real word has. A real word ("Hanks", "Wort")
/// may be meant as said, so only the audio can decide those: see `Term::sounds_like`.
pub fn apply_learned(text: &str, learned: &[Learned]) -> String {
    let rules: Vec<Replacement> = learned
        .iter()
        .filter(|l| !common(&l.from))
        .map(|l| Replacement {
            from: l.from.clone(),
            to: l.to.clone(),
        })
        .collect();
    replace(text, &rules)
}

/// Whether a word is one people really say, so that replacing it blindly would break sentences.
pub fn common(word: &str) -> bool {
    let word = word.trim().to_lowercase();
    word.chars().count() < 3
        || word.split_whitespace().count() > 1 && all_common(&word)
        || COMMON.contains(&word.as_str())
}
fn all_common(words: &str) -> bool {
    words.split_whitespace().all(|w| COMMON.contains(&w))
}

/// Numbers, days, months and the most frequent other German and English words.
const COMMON: &[&str] = &[
    "null",
    "eins",
    "zwei",
    "drei",
    "vier",
    "fünf",
    "sechs",
    "sieben",
    "acht",
    "neun",
    "zehn",
    "elf",
    "zwölf",
    "zwanzig",
    "dreißig",
    "hundert",
    "tausend",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "montag",
    "dienstag",
    "mittwoch",
    "donnerstag",
    "freitag",
    "samstag",
    "sonntag",
    "januar",
    "februar",
    "märz",
    "april",
    "mai",
    "juni",
    "juli",
    "august",
    "september",
    "oktober",
    "november",
    "dezember",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "aber",
    "about",
    "after",
    "alle",
    "allem",
    "allen",
    "aller",
    "alles",
    "als",
    "also",
    "alt",
    "and",
    "andere",
    "anderen",
    "anders",
    "any",
    "arbeit",
    "are",
    "auch",
    "auf",
    "aus",
    "back",
    "bald",
    "because",
    "been",
    "before",
    "bei",
    "beim",
    "bereits",
    "besser",
    "beste",
    "bild",
    "bin",
    "bis",
    "bist",
    "bitte",
    "bleiben",
    "both",
    "brauchen",
    "but",
    "call",
    "can",
    "come",
    "could",
    "dabei",
    "dafür",
    "dagegen",
    "daher",
    "damit",
    "danach",
    "dann",
    "daran",
    "darauf",
    "das",
    "dass",
    "datei",
    "davon",
    "dazu",
    "dein",
    "deine",
    "deinen",
    "dem",
    "den",
    "denn",
    "der",
    "des",
    "deutlich",
    "dich",
    "did",
    "die",
    "dies",
    "diese",
    "diesem",
    "diesen",
    "dieser",
    "dieses",
    "dinge",
    "dir",
    "doch",
    "does",
    "done",
    "dort",
    "down",
    "drei",
    "durch",
    "dürfen",
    "each",
    "eben",
    "ein",
    "eine",
    "einem",
    "einen",
    "einer",
    "eines",
    "einfach",
    "einmal",
    "einige",
    "ende",
    "er",
    "erste",
    "erst",
    "es",
    "etwas",
    "euch",
    "euer",
    "even",
    "ever",
    "every",
    "fall",
    "falsch",
    "fast",
    "fertig",
    "file",
    "find",
    "first",
    "for",
    "form",
    "frage",
    "from",
    "für",
    "ganz",
    "gar",
    "geben",
    "gehen",
    "geht",
    "genau",
    "gerade",
    "gern",
    "gerne",
    "get",
    "gibt",
    "give",
    "gleich",
    "good",
    "got",
    "groß",
    "große",
    "gut",
    "gute",
    "guten",
    "haben",
    "had",
    "halt",
    "has",
    "hast",
    "hat",
    "hatte",
    "have",
    "heißt",
    "her",
    "here",
    "heute",
    "hier",
    "him",
    "his",
    "hoch",
    "how",
    "ich",
    "ihm",
    "ihn",
    "ihnen",
    "ihr",
    "ihre",
    "ihren",
    "immer",
    "ins",
    "into",
    "ist",
    "its",
    "ja",
    "jahr",
    "jede",
    "jeden",
    "jeder",
    "jedes",
    "jetzt",
    "just",
    "kann",
    "kannst",
    "kein",
    "keine",
    "keinen",
    "keiner",
    "klar",
    "know",
    "kommen",
    "kommt",
    "können",
    "könnte",
    "kurz",
    "lang",
    "lange",
    "lassen",
    "last",
    "leben",
    "leider",
    "let",
    "letzte",
    "leute",
    "like",
    "line",
    "list",
    "little",
    "long",
    "look",
    "los",
    "machen",
    "macht",
    "made",
    "make",
    "man",
    "manche",
    "mal",
    "many",
    "markt",
    "may",
    "mehr",
    "mein",
    "meine",
    "meinem",
    "meinen",
    "meiner",
    "mich",
    "mir",
    "mit",
    "more",
    "morgen",
    "most",
    "much",
    "muss",
    "müssen",
    "must",
    "name",
    "nach",
    "nächste",
    "name",
    "natürlich",
    "need",
    "neben",
    "nehmen",
    "nein",
    "neu",
    "neue",
    "neuen",
    "new",
    "nicht",
    "nichts",
    "nie",
    "noch",
    "not",
    "now",
    "nun",
    "nur",
    "oben",
    "ober",
    "oder",
    "off",
    "ohne",
    "okay",
    "old",
    "one",
    "only",
    "other",
    "our",
    "out",
    "over",
    "paar",
    "part",
    "people",
    "plan",
    "richtig",
    "right",
    "sache",
    "sachen",
    "sagen",
    "sagt",
    "said",
    "same",
    "satz",
    "schon",
    "schön",
    "schnell",
    "sehen",
    "sehr",
    "sein",
    "seine",
    "seinem",
    "seinen",
    "seiner",
    "seit",
    "selbst",
    "set",
    "she",
    "should",
    "sich",
    "sie",
    "sind",
    "soll",
    "sollen",
    "sollte",
    "some",
    "sonst",
    "spiel",
    "sprache",
    "start",
    "stehen",
    "steht",
    "still",
    "take",
    "tag",
    "tage",
    "tagen",
    "test",
    "text",
    "than",
    "that",
    "the",
    "their",
    "them",
    "then",
    "there",
    "these",
    "they",
    "thing",
    "think",
    "this",
    "those",
    "time",
    "two",
    "über",
    "uhr",
    "und",
    "uns",
    "unser",
    "unsere",
    "unter",
    "use",
    "very",
    "viel",
    "viele",
    "vielen",
    "vielleicht",
    "vier",
    "vom",
    "von",
    "vor",
    "wann",
    "want",
    "war",
    "waren",
    "warum",
    "was",
    "way",
    "weg",
    "weil",
    "weiter",
    "weitere",
    "welche",
    "welt",
    "wenig",
    "wenn",
    "wer",
    "werden",
    "werk",
    "wert",
    "what",
    "when",
    "where",
    "which",
    "who",
    "wie",
    "wieder",
    "will",
    "willst",
    "wir",
    "wird",
    "wirklich",
    "wissen",
    "with",
    "wo",
    "woche",
    "wohl",
    "wollen",
    "work",
    "world",
    "worte",
    "worten",
    "wort",
    "wörter",
    "would",
    "wurde",
    "würde",
    "year",
    "you",
    "your",
    "zeit",
    "ziel",
    "zum",
    "zur",
    "zurück",
    "zusammen",
    "zwar",
    "zwei",
    "zwischen",
];

/// Kölner Phonetik: a code for how a German word sounds. It treats v, f and w alike and drops
/// vowels, so "Vorto", "Forto" and "Warto" all come out as 372.
pub fn phonetic(word: &str) -> String {
    let letters: Vec<char> = word
        .to_lowercase()
        .chars()
        .flat_map(|c| match c {
            'ä' => vec!['a'],
            'ö' => vec!['o'],
            'ü' => vec!['u'],
            'ß' => vec!['s'],
            c if c.is_alphabetic() => vec![c],
            _ => vec![],
        })
        .collect();
    let mut code = String::new();
    for (i, &c) in letters.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| letters[p]);
        let next = letters.get(i + 1).copied();
        let digit = match c {
            'a' | 'e' | 'i' | 'j' | 'o' | 'u' | 'y' => "0",
            'h' => "",
            'b' => "1",
            'p' => {
                if next == Some('h') {
                    "3"
                } else {
                    "1"
                }
            }
            'd' | 't' => {
                if matches!(next, Some('c' | 's' | 'z')) {
                    "8"
                } else {
                    "2"
                }
            }
            'f' | 'v' | 'w' => "3",
            'g' | 'k' | 'q' => "4",
            'c' => {
                let hard = |n| matches!(n, Some('a' | 'h' | 'k' | 'o' | 'q' | 'u' | 'x'));
                if i == 0 {
                    if matches!(
                        next,
                        Some('a' | 'h' | 'k' | 'l' | 'o' | 'q' | 'r' | 'u' | 'x')
                    ) {
                        "4"
                    } else {
                        "8"
                    }
                } else if matches!(prev, Some('s' | 'z')) || !hard(next) {
                    "8"
                } else {
                    "4"
                }
            }
            'x' => {
                if matches!(prev, Some('c' | 'k' | 'q')) {
                    "8"
                } else {
                    "48"
                }
            }
            'l' => "5",
            'm' | 'n' => "6",
            'r' => "7",
            's' | 'z' => "8",
            _ => "",
        };
        code.push_str(digit);
    }
    // Neighbours collapse, then the vowels go.
    let mut out = String::new();
    let mut last = None;
    for c in code.chars() {
        if last != Some(c) && c != '0' {
            out.push(c);
        }
        last = Some(c);
    }
    out
}

fn distance(a: &[char], b: &[char]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut last = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let cost = last + usize::from(x != y);
            last = row[j + 1];
            row[j + 1] = cost.min(last + 1).min(row[j] + 1);
        }
    }
    row[b.len()]
}

/// Lowercase letters and digits only, so "open code," compares equal to "OpenCode".
fn plain(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Whether two spellings could be the same spoken word.
pub fn sounds_alike(a: &str, b: &str) -> bool {
    let (x, y) = (plain(a), plain(b));
    if x.is_empty() || y.is_empty() {
        return false;
    }
    let (cx, cy): (Vec<char>, Vec<char>) = (x.chars().collect(), y.chars().collect());
    let longest = cx.len().max(cy.len());
    let letters = distance(&cx, &cy);
    let (px, py): (Vec<char>, Vec<char>) = (
        phonetic(&x).chars().collect(),
        phonetic(&y).chars().collect(),
    );
    let sounds = distance(&px, &py);
    if px.is_empty() || py.is_empty() {
        return letters * 3 <= longest;
    }
    // Few sounds leave little to compare, so short words have to match exactly.
    let allowed = usize::from(px.len().max(py.len()) >= 5);
    (sounds <= allowed && letters * 3 <= longest * 2) || letters * 4 <= longest
}

/// Words the recognizer wrote that may really be one of the terms.
#[derive(Debug, PartialEq, Clone)]
pub struct Candidate {
    /// The first word of the span.
    pub word: usize,
    /// How many words the span covers.
    pub len: usize,
    pub term: usize,
}

/// At most this many spans get checked per recognition, to bound the time it takes.
pub const MAX_CANDIDATES: usize = 12;

/// Spans of up to three words that are spelled or sound like a term. Exact spellings of
/// the term itself, and addresses and paths, are left alone.
pub fn candidates(words: &[&str], terms: &[Term]) -> Vec<Candidate> {
    let mut found = Vec::new();
    let mut i = 0;
    'words: while i < words.len() {
        if found.len() == MAX_CANDIDATES {
            break;
        }
        // The shortest span that fits, so a term doesn't swallow the word after it.
        for len in 1..=3 {
            if i + len > words.len() {
                continue;
            }
            let span = &words[i..i + len];
            if span.iter().any(|w| w.contains(['/', '\\', '@', ':', '_'])) {
                continue;
            }
            let joined = plain(&span.concat());
            if joined.is_empty() {
                continue;
            }
            for (t, term) in terms.iter().enumerate() {
                let wanted = plain(&term.term);
                if wanted == joined && len == 1 {
                    continue;
                }
                // "open code" is the term in two words; only the spelling differs.
                let split = wanted == joined;
                // A span of several words has to be the mistake as a whole: "the Svelte" is the
                // term after an ordinary word, not a way of mishearing it.
                let ordinary = len > 1
                    && span.iter().any(|w| {
                        let word = plain(w);
                        word == wanted || COMMON.contains(&word.as_str())
                    });
                if joined.chars().count() * 2 > wanted.chars().count() * 3 + 2 {
                    continue;
                }
                let heard_before = term.sounds_like.iter().any(|s| plain(s) == joined);
                // Short names sound like too many words unless the user saw this mistake.
                let long_enough = wanted.chars().count() >= 4;
                // An inflection of the term is the term: "Tauris" isn't a misheard "Tauri".
                let inflected = joined.starts_with(&wanted)
                    && joined.chars().count() <= wanted.chars().count() + 2;
                if split
                    || heard_before
                    || (!ordinary && long_enough && !inflected && sounds_alike(&joined, &wanted))
                {
                    found.push(Candidate {
                        word: i,
                        len,
                        term: t,
                    });
                    i += len;
                    continue 'words;
                }
            }
        }
        i += 1;
    }
    found
}

/// Puts `term` where the words `word..word + len` were, keeping the punctuation around them.
pub fn swap(words: &[&str], candidate: &Candidate, term: &str) -> Vec<String> {
    let end = candidate.word + candidate.len;
    let lead: String = words[candidate.word]
        .chars()
        .take_while(|c| !c.is_alphanumeric())
        .collect();
    let trail: String = {
        let tail: Vec<char> = words[end - 1]
            .chars()
            .rev()
            .take_while(|c| !c.is_alphanumeric())
            .collect();
        tail.into_iter().rev().collect()
    };
    let mut out: Vec<String> = words[..candidate.word]
        .iter()
        .map(|w| w.to_string())
        .collect();
    out.push(format!("{lead}{term}{trail}"));
    out.extend(words[end..].iter().map(|w| w.to_string()));
    out
}

/// Fixes the case of a term where only the case is off, for terms whose spelling shows they're
/// names: "github" and "Github" become "GitHub". Plain words keep the case the model gave them.
pub fn fix_case(text: &str, vocabulary: &[String]) -> String {
    let rules: Vec<Replacement> = vocabulary
        .iter()
        .filter(|w| {
            let inner = w.chars().skip(1).any(char::is_uppercase);
            inner && !common(w)
        })
        .map(|w| Replacement {
            from: w.clone(),
            to: w.clone(),
        })
        .collect();
    replace(text, &rules)
}

/// One word or short phrase the user replaced, as written by Vorto and as the user wants it.
#[derive(Debug, PartialEq, Clone)]
pub struct Correction {
    pub from: String,
    pub to: String,
}

fn bare(word: &str) -> &str {
    word.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Compares what Vorto wrote with what the user changed it to, and keeps the changes that look
/// like a misheard word. Rewrites, added or removed words, grammar and punctuation teach nothing.
pub fn corrections(before: &str, after: &str) -> Vec<Correction> {
    let old: Vec<&str> = before.split_whitespace().collect();
    let new: Vec<&str> = after.split_whitespace().collect();
    if old.is_empty() || new.is_empty() || old.len() > 400 || new.len() > 400 {
        return Vec::new();
    }
    let hunks = diff(&old, &new);
    let changed: usize = hunks.iter().map(|h| h.1.len().max(h.3.len())).sum();
    // Most of it changed: that's a new sentence, not a correction.
    if changed * 2 > old.len().max(1).max(3) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (_, from, _, to) in hunks {
        if let Some(correction) = correction(from, to) {
            out.push(correction);
        }
    }
    out
}

fn correction(from: &[&str], to: &[&str]) -> Option<Correction> {
    if from.is_empty() || to.is_empty() || from.len() > 3 || to.len() > 3 {
        return None;
    }
    let heard = from.iter().map(|w| bare(w)).collect::<Vec<_>>().join(" ");
    let wanted = to.iter().map(|w| bare(w)).collect::<Vec<_>>().join(" ");
    let (a, b) = (plain(&heard), plain(&wanted));
    if a.is_empty() || b.is_empty() || heard == wanted {
        return None;
    }
    // Numbers aren't misheard names.
    if a.chars().all(char::is_numeric) || b.chars().all(char::is_numeric) {
        return None;
    }
    if a == b {
        // Only the case or the spacing changed. That says something for a name such as GitHub,
        // and nothing at the start of a sentence.
        let shaped = wanted.chars().skip(1).any(char::is_uppercase)
            || wanted.contains(' ') != heard.contains(' ');
        return shaped.then_some(Correction {
            from: heard,
            to: wanted,
        });
    }
    // Grammar: one form of a word became another ("Haus" to "Hauses", "ging" to "gingen").
    let (shorter, longer) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    if longer.starts_with(shorter.as_str()) && longer.chars().count() - shorter.chars().count() <= 3
    {
        return None;
    }
    if !could_be_misheard(&a, &b) {
        return None;
    }
    // Replacing one everyday word by another is usually a change of mind.
    if common(&heard) && common(&wanted) {
        return None;
    }
    Some(Correction {
        from: heard,
        to: wanted,
    })
}

/// More forgiving than `sounds_alike`: the user has told us that it's the same word, we only
/// have to rule out that they meant a different one.
fn could_be_misheard(a: &str, b: &str) -> bool {
    let (ca, cb): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (pa, pb): (Vec<char>, Vec<char>) =
        (phonetic(a).chars().collect(), phonetic(b).chars().collect());
    let longest_sounds = pa.len().max(pb.len()).max(1);
    distance(&pa, &pb) * 2 <= longest_sounds || distance(&ca, &cb) * 3 <= ca.len().max(cb.len())
}

/// The stretches that differ between two word lists: (position in `old`, removed words,
/// position in `new`, added words). Punctuation and case alone don't count as a difference
/// here, so "Hallo," and "hallo" line up.
fn diff<'a>(
    old: &'a [&'a str],
    new: &'a [&'a str],
) -> Vec<(usize, &'a [&'a str], usize, &'a [&'a str])> {
    let eq = |a: &str, b: &str| a == b || (plain(a) == plain(b) && !shape_differs(a, b));
    let (n, m) = (old.len(), new.len());
    let mut table = vec![0u16; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * (m + 1) + j] = if eq(old[i], new[j]) {
                table[(i + 1) * (m + 1) + j + 1] + 1
            } else {
                table[(i + 1) * (m + 1) + j].max(table[i * (m + 1) + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut hunks = Vec::new();
    while i < n || j < m {
        if i < n && j < m && eq(old[i], new[j]) {
            i += 1;
            j += 1;
            continue;
        }
        let (start_i, start_j) = (i, j);
        while (i < n || j < m) && !(i < n && j < m && eq(old[i], new[j])) {
            if j == m || (i < n && table[(i + 1) * (m + 1) + j] >= table[i * (m + 1) + j + 1]) {
                i += 1;
            } else {
                j += 1;
            }
        }
        hunks.push((start_i, &old[start_i..i], start_j, &new[start_j..j]));
    }
    hunks
}

/// "github" to "GitHub" is a correction worth seeing; "hallo" to "Hallo" is not.
fn shape_differs(a: &str, b: &str) -> bool {
    let inner = |w: &str| bare(w).chars().skip(1).any(char::is_uppercase);
    bare(a) != bare(b) && (inner(a) || inner(b))
}

/// Where a dictation sits in the text of a field the user may have edited since. Returns the
/// words as they read now, or `None` when the dictation can't be found any more.
pub fn locate(inserted: &str, document: &str) -> Option<String> {
    let needle: Vec<&str> = inserted.split_whitespace().collect();
    let hay: Vec<&str> = document.split_whitespace().collect();
    if needle.is_empty() || hay.is_empty() || needle.len() > 300 {
        return None;
    }
    let (n, m) = (needle.len(), hay.len());
    // Edit distance of the dictation against any stretch of the document; `from` follows where
    // each stretch began.
    let mut cost: Vec<usize> = vec![0; m + 1];
    let mut from: Vec<usize> = (0..=m).collect();
    for (i, word) in needle.iter().enumerate() {
        let (mut diagonal, mut diagonal_from) = (cost[0], from[0]);
        cost[0] = i + 1;
        for j in 1..=m {
            let same = plain(word) == plain(hay[j - 1]);
            let (above, above_from) = (cost[j], from[j]);
            let substitute = diagonal + usize::from(!same);
            let (mut best, mut best_from) = (substitute, diagonal_from);
            if above + 1 < best {
                (best, best_from) = (above + 1, above_from);
            }
            if cost[j - 1] + 1 < best {
                (best, best_from) = (cost[j - 1] + 1, from[j - 1]);
            }
            (diagonal, diagonal_from) = (above, above_from);
            cost[j] = best;
            from[j] = best_from;
        }
        from[0] = 0;
    }
    // The stretch nearest the caret wins a tie: new text is usually further down.
    let (end, &best) = cost.iter().enumerate().rev().min_by_key(|(_, c)| **c)?;
    if best * 2 > n {
        return None;
    }
    Some(hay[from[end]..end].join(" "))
}

/// Adds what the user corrected, and counts it again when it's known. Returns the new ones.
/// A correction that takes back an earlier one replaces it.
pub fn remember(learned: &mut Vec<Learned>, found: Vec<Correction>) -> Vec<Correction> {
    let mut fresh = Vec::new();
    for c in found {
        learned.retain(|l| {
            !(l.from.eq_ignore_ascii_case(&c.to) && l.to.eq_ignore_ascii_case(&c.from))
        });
        match learned
            .iter()
            .position(|l| l.from.eq_ignore_ascii_case(&c.from) && l.to == c.to)
        {
            Some(i) => {
                let mut known = learned.remove(i);
                known.count += 1;
                learned.push(known);
            }
            None => {
                learned.push(Learned {
                    from: c.from.clone(),
                    to: c.to.clone(),
                    count: 1,
                });
                fresh.push(c);
            }
        }
    }
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(from: &str, to: &str) -> Replacement {
        Replacement {
            from: from.into(),
            to: to.into(),
        }
    }
    fn learned(from: &str, to: &str) -> Learned {
        Learned {
            from: from.into(),
            to: to.into(),
            count: 1,
        }
    }
    fn pair(c: &Correction) -> (&str, &str) {
        (&c.from, &c.to)
    }
    #[test]
    fn replaces_whole_words_ignoring_case() {
        let rules = [r("vortho", "Vorto"), r("tauri zwei", "Tauri 2")];
        assert_eq!(
            replace("Vortho läuft mit tauri Zwei, vorthos nicht.", &rules),
            "Vorto läuft mit Tauri 2, vorthos nicht."
        );
    }
    #[test]
    fn line_breaks_can_be_dictated() {
        let rules = [r("new paragraph", "\\n\\n")];
        assert_eq!(
            replace("Hi Anna, new paragraph thanks for the call.", &rules),
            "Hi Anna,\n\nthanks for the call."
        );
    }
    #[test]
    fn whisper_prompt_lists_words() {
        assert_eq!(
            whisper_prompt(&["Vorto".into(), "Tauri".into()]),
            "Vorto, Tauri."
        );
        assert_eq!(whisper_prompt(&[]), "");
    }
    #[test]
    fn suggests_names_and_terms() {
        let entry = |text: &str| Entry {
            text: text.into(),
            app: String::new(),
            at: String::new(),
            seconds: 1.0,
            raw: String::new(),
        };
        let history = [
            entry("Push it to GitHub and ask the API team."),
            entry("GitHub is down, the API too."),
            entry("Das ist gut."),
        ];
        let words = suggestions(&history, &[], &["api".into()]);
        assert_eq!(words, ["GitHub"]);
        assert!(suggestions(&history, &["github".into()], &[]).contains(&"API".to_string()));
    }
    #[test]
    fn common_words_are_lowercase_single_words() {
        assert!(COMMON
            .iter()
            .all(|w| *w == w.to_lowercase() && !w.contains(' ')));
    }
    #[test]
    fn phonetics_group_what_sounds_alike() {
        assert_eq!(phonetic("Vorto"), "372");
        assert_eq!(phonetic("Warto"), "372");
        assert_eq!(phonetic("Wortto"), "372");
        assert_eq!(phonetic("Forto"), "372");
        assert_eq!(phonetic("shadcn"), phonetic("Schatz CN"));
        assert_eq!(phonetic("Müller"), phonetic("Mueller"));
        assert_ne!(phonetic("Vorto"), phonetic("Tisch"));
    }
    #[test]
    fn finds_words_that_sound_like_a_term() {
        let terms = [Term {
            term: "Vorto".into(),
            sounds_like: vec![],
        }];
        let words = ["Mit", "Warto", "bin", "ich", "zufrieden."];
        assert_eq!(
            candidates(&words, &terms),
            [Candidate {
                word: 1,
                len: 1,
                term: 0
            }]
        );
        // A real word can be a candidate too: only the audio tells them apart.
        assert_eq!(candidates(&["Das", "Wort", "stimmt."], &terms).len(), 1);
        assert!(candidates(&["Das", "Tischbein", "stimmt."], &terms).is_empty());
        assert!(candidates(&["Mit", "Vorto", "geht's."], &terms).is_empty());
        assert!(candidates(&["siehe", "c:/warto"], &terms).is_empty());
    }
    #[test]
    fn finds_terms_split_over_several_words() {
        let terms = [Term {
            term: "shadcn".into(),
            sounds_like: vec![],
        }];
        let words = ["Mehr", "Schatz", "CN", "Styling."];
        assert_eq!(
            candidates(&words, &terms),
            [Candidate {
                word: 1,
                len: 2,
                term: 0
            }]
        );
    }
    #[test]
    fn a_term_in_two_words_is_found_without_the_word_after_it() {
        let terms = [Term {
            term: "OpenCode".into(),
            sounds_like: vec![],
        }];
        assert_eq!(
            candidates(&["von", "Open", "Code", "bei", "uns."], &terms),
            [Candidate {
                word: 1,
                len: 2,
                term: 0
            }]
        );
    }
    #[test]
    fn a_term_keeps_the_ordinary_word_before_it() {
        let terms = [Term {
            term: "Svelte".into(),
            sounds_like: vec![],
        }];
        assert!(candidates(&["The", "Svelte", "frontend."], &terms).is_empty());
        assert!(candidates(&["Das", "Zwelt", "ist."], &terms).len() == 1);
    }
    #[test]
    fn short_terms_need_a_known_mistake() {
        let mut terms = [Term {
            term: "Hex".into(),
            sounds_like: vec![],
        }];
        assert!(candidates(&["Tom", "Hanks", "lacht."], &terms).is_empty());
        terms[0].sounds_like = vec!["Hanks".into()];
        assert_eq!(candidates(&["Tom", "Hanks", "lacht."], &terms).len(), 1);
    }
    #[test]
    fn inflections_of_a_term_stay() {
        let terms = [Term {
            term: "Tauri".into(),
            sounds_like: vec![],
        }];
        assert!(candidates(&["Die", "Tauris", "sind", "da."], &terms).is_empty());
    }
    #[test]
    fn swapping_keeps_punctuation() {
        let words = ["Mit", "(Warto),", "gut."];
        let c = Candidate {
            word: 1,
            len: 1,
            term: 0,
        };
        assert_eq!(swap(&words, &c, "Vorto").join(" "), "Mit (Vorto), gut.");
        let words = ["Mehr", "Schatz", "CN."];
        let c = Candidate {
            word: 1,
            len: 2,
            term: 0,
        };
        assert_eq!(swap(&words, &c, "shadcn").join(" "), "Mehr shadcn.");
    }
    #[test]
    fn learns_a_misheard_word() {
        let found = corrections(
            "Ich bin mit Warto nicht so zufrieden.",
            "Ich bin mit Vorto nicht so zufrieden.",
        );
        assert_eq!(
            found.iter().map(pair).collect::<Vec<_>>(),
            [("Warto", "Vorto")]
        );
    }
    #[test]
    fn learns_a_phrase_that_became_one_word() {
        let found = corrections(
            "Mehr Schatz CN Styling bitte.",
            "Mehr shadcn Styling bitte.",
        );
        assert_eq!(
            found.iter().map(pair).collect::<Vec<_>>(),
            [("Schatz CN", "shadcn")]
        );
    }
    #[test]
    fn learns_how_a_name_is_written() {
        let found = corrections("Das liegt auf github heute.", "Das liegt auf GitHub heute.");
        assert_eq!(
            found.iter().map(pair).collect::<Vec<_>>(),
            [("github", "GitHub")]
        );
    }
    #[test]
    fn grammar_and_punctuation_teach_nothing() {
        assert!(corrections("Das Ende des Haus ist da.", "Das Ende des Hauses ist da.").is_empty());
        assert!(corrections("Hallo wie geht es dir", "Hallo, wie geht es dir?").is_empty());
        assert!(corrections("hallo wie geht es dir", "Hallo wie geht es dir").is_empty());
    }
    #[test]
    fn a_change_of_mind_teaches_nothing() {
        assert!(corrections(
            "Wir treffen uns am Montag um zehn.",
            "Wir sehen uns am Dienstag um neun."
        )
        .is_empty());
        assert!(corrections(
            "Das ist ein Hund im Garten.",
            "Das ist eine Katze im Garten."
        )
        .is_empty());
    }
    #[test]
    fn a_rewrite_teaches_nothing() {
        assert!(corrections(
            "Bitte ruf mich morgen an.",
            "Melde dich doch einfach, wenn es passt."
        )
        .is_empty());
    }
    #[test]
    fn added_and_removed_words_teach_nothing() {
        assert!(corrections("Das ist gut.", "Das ist wirklich gut.").is_empty());
        assert!(corrections("Das ist wirklich gut.", "Das ist gut.").is_empty());
    }
    #[test]
    fn uncommon_spellings_are_replaced_and_real_words_are_not() {
        let list = [learned("Warto", "Vorto"), learned("Wort", "Vorto")];
        assert_eq!(
            apply_learned("Warto und das Wort.", &list),
            "Vorto und das Wort."
        );
    }
    #[test]
    fn terms_put_corrections_first_and_merge_spellings() {
        let list = [
            learned("Warto", "Vorto"),
            learned("Wortto", "Vorto"),
            learned("Hanks", "Hex"),
        ];
        let found = terms(&["Vorto".into(), "Tauri".into()], &list);
        assert_eq!(found[0].term, "Hex");
        assert_eq!(found[1].term, "Vorto");
        assert_eq!(found[1].sounds_like, ["Wortto", "Warto"]);
        assert_eq!(found[2].term, "Tauri");
        assert_eq!(found.len(), 3);
    }
    #[test]
    fn remembering_counts_and_takes_back() {
        let mut list = Vec::new();
        let c = |a: &str, b: &str| Correction {
            from: a.into(),
            to: b.into(),
        };
        assert_eq!(remember(&mut list, vec![c("Warto", "Vorto")]).len(), 1);
        assert!(remember(&mut list, vec![c("Warto", "Vorto")]).is_empty());
        assert_eq!(list[0].count, 2);
        remember(&mut list, vec![c("Vorto", "Warto")]);
        assert_eq!(list, [learned("Vorto", "Warto")]);
    }
    #[test]
    fn locates_an_edited_dictation() {
        let doc = "Lieber Paul,\nmit Vorto bin ich zufrieden, wirklich.\nViele Grüße";
        assert_eq!(
            locate("mit Warto bin ich zufrieden, wirklich.", doc).as_deref(),
            Some("mit Vorto bin ich zufrieden, wirklich.")
        );
        assert_eq!(locate("etwas ganz anderes hier geschrieben", doc), None);
    }
    #[test]
    fn locates_the_last_of_two_copies() {
        let doc = "eins zwei drei vier ... eins zwei drei fünf";
        assert_eq!(
            locate("eins zwei drei fünf", doc).as_deref(),
            Some("eins zwei drei fünf")
        );
    }
    #[test]
    fn name_case_is_fixed_only_for_shaped_terms() {
        let words = ["GitHub".to_string(), "Sara".to_string()];
        assert_eq!(
            fix_case("auf github mit sara", &words),
            "auf GitHub mit sara"
        );
    }
}
