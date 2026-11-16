//! The naming rules behind Add Multiple, Rename and Find/Replace: `Vixen.Core/Rule/Name` and the algorithms of `NameGenerator`
//! and `SubstitutionRenamer`.

use std::collections::HashSet;

/// `INamingGenerator`, one per rule type (F136).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamingRule {
    /// `NumericCounter`.
    Numeric { start: i32, end: i32, endless: bool, step: i32 },
    /// `LetterCounter`.
    Letters { start: char, count: i32 },
    /// `LetterIterator`. Holds only letters and digits.
    LetterList(String),
    /// `WordIterator`.
    WordList(Vec<String>),
}

/// The alphabet of `LetterCounter`, with 'a' twice. That repeat is why A..Z is followed by AA, AA, AB (kept as upstream has it).
const LETTERS: &[u8] = b"aabcdefghijklmnopqrstuvwxyz";

impl NamingRule {
    /// A fresh rule of each type, as the "+" button adds it.
    pub fn new_numeric() -> NamingRule {
        NamingRule::Numeric { start: 1, end: 5, endless: true, step: 1 }
    }

    pub fn new_letters() -> NamingRule {
        NamingRule::Letters { start: 'A', count: 5 }
    }

    pub fn new_letter_list() -> NamingRule {
        NamingRule::LetterList("ABCDE".to_string())
    }

    pub fn new_word_list() -> NamingRule {
        NamingRule::WordList(Vec::new())
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            NamingRule::Numeric { .. } => "Sequential Numbers",
            NamingRule::Letters { .. } => "Sequential Letters",
            NamingRule::LetterList(_) => "List of Letters",
            NamingRule::WordList(_) => "List of Words",
        }
    }

    /// True when the rule never runs out.
    pub fn is_endless(&self) -> bool {
        match *self {
            NamingRule::Numeric { start, end, endless, step } => {
                // "step and range disagree in sign" also counts as endless.
                endless || (step > 0 && end < start) || (step < 0 && end > start) || step == 0
            }
            _ => false,
        }
    }

    /// The cycle length. `i32::MAX` for an endless rule.
    pub fn iterations(&self) -> i32 {
        if self.is_endless() {
            return i32::MAX;
        }
        match self {
            NamingRule::Numeric { start, end, step, .. } => {
                let range = i64::from(*end) - i64::from(*start);
                (range / i64::from(*step) + 1).clamp(0, i64::from(i32::MAX)) as i32
            }
            NamingRule::Letters { count, .. } => *count,
            NamingRule::LetterList(letters) => letters.chars().count() as i32,
            NamingRule::WordList(words) => words.len() as i32,
        }
    }

    /// `GenerateName(i)`.
    pub fn name_at(&self, i: i32) -> String {
        match self {
            NamingRule::Numeric { start, step, .. } => {
                (i64::from(*start) + i64::from(*step) * i64::from(i)).to_string()
            }
            NamingRule::Letters { start, .. } => letter_name(*start, i),
            NamingRule::LetterList(letters) => {
                let count = letters.chars().count() as i32;
                if count == 0 {
                    return String::new();
                }
                letters.chars().nth((i.rem_euclid(count)) as usize).map(String::from).unwrap_or_default()
            }
            NamingRule::WordList(words) => {
                if words.is_empty() {
                    return String::new();
                }
                words[i.rem_euclid(words.len() as i32) as usize].clone()
            }
        }
    }
}

/// `LetterCounter`: base 27 over `LETTERS`, from the start letter's index + 1 + `i`. Case comes from the start letter.
fn letter_name(start: char, i: i32) -> String {
    let start_index = i64::from((start.to_ascii_lowercase() as u8).wrapping_sub(b'a'));
    let mut value = start_index + 1 + i64::from(i);
    let mut out = Vec::new();
    while value > 0 {
        out.push(LETTERS[(value % 27) as usize]);
        value /= 27;
    }
    out.reverse();
    let text = String::from_utf8(out).unwrap_or_default();
    if start.is_ascii_uppercase() {
        text.to_ascii_uppercase()
    } else {
        text
    }
}

/// `LetterIterator`'s setter: only `[A-Za-z0-9]` stays.
pub fn clean_letter_list(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphanumeric).collect()
}

/// `WordIteratorEditor`: split on commas, each trimmed, with empty entries kept. So blank text is one empty word, which is what an untouched rule's editor
/// leaves it holding, and the rule then has one iteration that substitutes nothing (F136).
pub fn split_word_list(text: &str) -> Vec<String> {
    text.split(',').map(|w| w.trim().to_string()).collect()
}

/// `NumericCounterEditor`: a step never becomes 0. Typing 0 jumps to -1, or to 1 if the previous step was negative.
pub fn numeric_step_after_edit(typed: i32, previous: i32) -> i32 {
    if typed != 0 {
        typed
    } else if previous < 0 {
        1
    } else {
        -1
    }
}

/// `GenerateNames` (F135). Rules behave like an odometer with the last one fastest.
pub fn generate_names(format: &str, rules: &[NamingRule], wanted: usize) -> Vec<String> {
    let mut out = Vec::new();
    if rules.is_empty() || wanted == 0 {
        return out;
    }
    expand(format, rules, 0, wanted, &mut out);
    out
}

fn expand(format: &str, rules: &[NamingRule], depth: usize, wanted: usize, out: &mut Vec<String>) {
    let Some(rule) = rules.get(depth) else { return };
    let placeholder = format!("<{}>", depth + 1);
    let iterations = rule.iterations();
    let mut i = 0;
    while i < iterations && out.len() < wanted {
        let text = format.replace(&placeholder, &rule.name_at(i));
        if depth + 1 == rules.len() {
            out.push(text);
        } else {
            let before = out.len();
            expand(&text, rules, depth + 1, wanted, out);
            if out.len() == before {
                out.push(text);
            }
        }
        i += 1;
    }
}

/// `SubstitutionRenamer`: ordinal, case-sensitive, all occurrences, patterns in order, each on the last one's result. Empty Finds are skipped.
pub fn replace_patterns(old: &[String], patterns: &[(String, String)]) -> Vec<String> {
    old.iter()
        .map(|name| {
            let mut text = name.clone();
            for (find, replace) in patterns {
                if !find.is_empty() {
                    text = text.replace(find.as_str(), replace);
                }
            }
            text
        })
        .collect()
}

/// `#{n} Replace '{find}' with '{replace}'`, or `#{n} Nothing to find.`
pub fn pattern_row_text(index: usize, find: &str, replace: &str) -> String {
    if find.is_empty() {
        format!("#{} Nothing to find.", index + 1)
    } else {
        format!("#{} Replace '{find}' with '{replace}'", index + 1)
    }
}

/// `NumberedNamingTemplate`, the only template: the format box becomes `{seed}-<1>`, with one endless count from 1.
pub fn numbered_items_template(seed: &str) -> (String, Vec<NamingRule>) {
    (format!("{seed}-<1>"), vec![NamingRule::Numeric { start: 1, end: 5, endless: true, step: 1 }])
}

/// The Add button: if the format has no `<n>` for the new rule yet, it gets ` <n>` on the end.
pub fn format_after_add(format: &str, new_rule_number: usize) -> String {
    let placeholder = format!("<{new_rule_number}>");
    if format.contains(&placeholder) {
        format.to_string()
    } else {
        format!("{format} {placeholder}")
    }
}

/// The Delete button: always strips the placeholder of the old highest rule number, with or without a leading space. The others
/// aren't renumbered, so deleting a rule from the middle shifts what they mean.
pub fn format_after_delete(format: &str, old_rule_count: usize) -> String {
    let placeholder = format!("<{old_rule_count}>");
    format.replace(&format!(" {placeholder}"), "").replace(&placeholder, "")
}

/// `NamingUtilities.Uniquify`: against a set of names taken once, up front.
pub fn uniquify_in(names: &HashSet<String>, name: &str) -> String {
    if !names.contains(name) {
        return name.to_string();
    }
    let mut counter = 2;
    loop {
        let candidate = format!("{name} - {counter}");
        if !names.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeric(start: i32, end: i32, endless: bool, step: i32) -> NamingRule {
        NamingRule::Numeric { start, end, endless, step }
    }

    #[test]
    fn uniquify_in_a_set() {
        let mut set = HashSet::new();
        assert_eq!(uniquify_in(&set, "foo"), "foo");
        set.insert("foo".to_string());
        assert_eq!(uniquify_in(&set, "foo"), "foo - 2");
        set.insert("foo - 2".to_string());
        assert_eq!(uniquify_in(&set, "foo"), "foo - 3");
    }

    #[test]
    fn numbered_names() {
        assert_eq!(generate_names("X-<1>", &[numeric(1, 5, true, 1)], 3), ["X-1", "X-2", "X-3"]);
    }

    #[test]
    fn rules_are_an_odometer_with_the_last_fastest() {
        let rules = [numeric(1, 2, false, 1), NamingRule::Letters { start: 'A', count: 2 }];
        assert_eq!(generate_names("T <1> <2>", &rules, 3), ["T 1 A", "T 1 B", "T 2 A"]);
        assert_eq!(generate_names("T <1> <2>", &rules, 10).len(), 4);
    }

    #[test]
    fn no_rules_means_no_names() {
        assert!(generate_names("X", &[], 5).is_empty());
    }

    #[test]
    fn a_format_with_no_placeholder_repeats() {
        assert_eq!(generate_names("X", &[numeric(1, 5, true, 1)], 2), ["X", "X"]);
    }

    #[test]
    fn an_exhausted_inner_rule_leaves_the_text_as_it_stands() {
        // A word list with nothing in it yields nothing, so each outer value stands alone with the placeholder still in it.
        let rules = [numeric(1, 2, false, 1), NamingRule::new_word_list()];
        assert_eq!(generate_names("<1> <2>", &rules, 5), ["1 <2>", "2 <2>"]);
    }

    #[test]
    fn numeric_iterations() {
        assert_eq!(numeric(1, 5, false, 1).iterations(), 5);
        assert_eq!(numeric(5, 1, false, -1).iterations(), 5);
        assert!(numeric(1, 5, false, -1).is_endless());
        assert_eq!(numeric(1, 6, false, 2).iterations(), 3);
        assert_eq!(numeric(1, 6, false, 2).name_at(2), "5");
    }

    #[test]
    fn letters_repeat_aa_as_upstream_does() {
        let r = NamingRule::Letters { start: 'A', count: 100 };
        assert_eq!(r.name_at(0), "A");
        assert_eq!(r.name_at(25), "Z");
        assert_eq!(r.name_at(26), "AA");
        assert_eq!(r.name_at(27), "AA");
        assert_eq!(r.name_at(28), "AB");
        assert_eq!(NamingRule::Letters { start: 'c', count: 1 }.name_at(0), "c");
    }

    #[test]
    fn letter_lists_keep_only_letters_and_digits_and_wrap() {
        assert_eq!(clean_letter_list("A b-1 "), "Ab1");
        assert_eq!(clean_letter_list("\u{e9}x"), "x");
        let r = NamingRule::LetterList("LR".to_string());
        assert_eq!(r.iterations(), 2);
        assert_eq!(r.name_at(3), "R");
    }

    #[test]
    fn word_lists_split_and_trim() {
        assert_eq!(split_word_list("Left, Right"), ["Left", "Right"]);
        assert_eq!(split_word_list("a,,b"), ["a", "", "b"]);
        assert_eq!(split_word_list(""), [""]);
        assert_eq!(split_word_list("  "), [""]);
        assert_eq!(NamingRule::new_word_list().iterations(), 0);
    }

    #[test]
    fn an_untouched_word_list_rule_substitutes_nothing() {
        let untouched = NamingRule::WordList(split_word_list(""));
        assert_eq!(generate_names("X-<1>", std::slice::from_ref(&untouched), 5), ["X-"]);
        assert_eq!(generate_names("X-<1> <2>", &[numeric(1, 5, true, 1), untouched], 2), ["X-1 ", "X-2 "]);
    }

    #[test]
    fn the_step_never_becomes_zero() {
        assert_eq!(numeric_step_after_edit(0, 1), -1);
        assert_eq!(numeric_step_after_edit(0, -1), 1);
        assert_eq!(numeric_step_after_edit(3, 1), 3);
    }

    #[test]
    fn find_and_replace() {
        let old = vec!["Tree 1".to_string()];
        let patterns = vec![("Tree".to_string(), "Arch".to_string()), ("1".to_string(), "2".to_string())];
        assert_eq!(replace_patterns(&old, &patterns), ["Arch 2"]);
        assert_eq!(replace_patterns(&old, &[(String::new(), "x".to_string())]), ["Tree 1"]);
        assert_eq!(replace_patterns(&old, &[("tree".to_string(), "x".to_string())]), ["Tree 1"]);
        assert_eq!(pattern_row_text(0, "", "x"), "#1 Nothing to find.");
        assert_eq!(pattern_row_text(1, "a", "b"), "#2 Replace 'a' with 'b'");
    }

    #[test]
    fn the_numbered_items_template() {
        let (format, rules) = numbered_items_template("Tree");
        assert_eq!(format, "Tree-<1>");
        assert_eq!(generate_names(&format, &rules, 2), ["Tree-1", "Tree-2"]);
    }

    #[test]
    fn adding_and_deleting_rules_edits_the_format() {
        assert_eq!(format_after_add("X-<1>", 2), "X-<1> <2>");
        assert_eq!(format_after_add("X-<1><2>", 2), "X-<1><2>");
        assert_eq!(format_after_delete("X-<1> <2>", 2), "X-<1>");
        assert_eq!(format_after_delete("X-<1><2>", 2), "X-<1>");
        // Deleting the first of two strips <2>, and <1> now means the old second rule.
        assert_eq!(format_after_delete("<1> <2>", 2), "<1>");
    }
}
