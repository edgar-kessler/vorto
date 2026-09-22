//! The user's dictionary: replacements applied to every dictation, the hint that helps Whisper
//! spell names, and words from History worth adding.
use crate::data::{Entry, Replacement};
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

#[cfg(test)]
mod tests {
    use super::*;
    fn r(from: &str, to: &str) -> Replacement {
        Replacement {
            from: from.into(),
            to: to.into(),
        }
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
}
