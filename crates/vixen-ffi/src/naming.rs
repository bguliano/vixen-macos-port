//! Shared pattern and substitution naming dialogs; API names retained for Setup compatibility.
use vixen_session::setup::naming::{
    clean_letter_list, format_after_add, format_after_delete, generate_names, numbered_items_template, numeric_step_after_edit, pattern_row_text,
    replace_patterns, split_word_list, NamingRule,
};

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum SetupRuleKind {
    SequentialNumbers,
    SequentialLetters,
    ListOfLetters,
    ListOfWords,
}

/// One naming rule, flat so the rule editors can bind to it.
#[derive(uniffi::Record, Clone)]
pub struct SetupNamingRule {
    pub kind: SetupRuleKind,
    pub start: i32,
    pub end: i32,
    pub endless: bool,
    pub step: i32,
    pub start_letter: String,
    pub count: i32,
    /// List of Letters' text, or List of Words' comma separated text.
    pub text: String,
}

fn rule_of(r: &SetupNamingRule) -> NamingRule {
    match r.kind {
        SetupRuleKind::SequentialNumbers => NamingRule::Numeric { start: r.start, end: r.end, endless: r.endless, step: r.step },
        SetupRuleKind::SequentialLetters => NamingRule::Letters { start: r.start_letter.chars().next().unwrap_or('A'), count: r.count },
        SetupRuleKind::ListOfLetters => NamingRule::LetterList(clean_letter_list(&r.text)),
        SetupRuleKind::ListOfWords => NamingRule::WordList(split_word_list(&r.text)),
    }
}

fn record_of(rule: &NamingRule) -> SetupNamingRule {
    let mut out = SetupNamingRule {
        kind: SetupRuleKind::SequentialNumbers,
        start: 1,
        end: 5,
        endless: true,
        step: 1,
        start_letter: "A".to_string(),
        count: 5,
        text: String::new(),
    };
    match rule {
        NamingRule::Numeric { start, end, endless, step } => {
            out.start = *start;
            out.end = *end;
            out.endless = *endless;
            out.step = *step;
        }
        NamingRule::Letters { start, count } => {
            out.kind = SetupRuleKind::SequentialLetters;
            out.start_letter = start.to_string();
            out.count = *count;
        }
        NamingRule::LetterList(text) => {
            out.kind = SetupRuleKind::ListOfLetters;
            out.text.clone_from(text);
        }
        NamingRule::WordList(words) => {
            out.kind = SetupRuleKind::ListOfWords;
            out.text = words.join(", ");
        }
    }
    out
}

#[derive(uniffi::Record, Clone)]
pub struct SetupFindReplace {
    pub find: String,
    pub replace: String,
}

#[derive(uniffi::Record)]
pub struct SetupNamingTemplate {
    pub format: String,
    pub rules: Vec<SetupNamingRule>,
}

#[uniffi::export]
pub fn setup_generate_names(format: String, rules: Vec<SetupNamingRule>, wanted: u32) -> Vec<String> {
    let rules: Vec<NamingRule> = rules.iter().map(rule_of).collect();
    generate_names(&format, &rules, wanted as usize)
}

#[uniffi::export]
pub fn setup_numbered_items_template(seed: String) -> SetupNamingTemplate {
    let (format, rules) = numbered_items_template(&seed);
    SetupNamingTemplate { format, rules: rules.iter().map(record_of).collect() }
}

/// A fresh rule of a kind, as "+" adds it.
#[uniffi::export]
pub fn setup_new_naming_rule(kind: SetupRuleKind) -> SetupNamingRule {
    record_of(&match kind {
        SetupRuleKind::SequentialNumbers => NamingRule::new_numeric(),
        SetupRuleKind::SequentialLetters => NamingRule::new_letters(),
        SetupRuleKind::ListOfLetters => NamingRule::new_letter_list(),
        SetupRuleKind::ListOfWords => NamingRule::new_word_list(),
    })
}

#[uniffi::export]
pub fn setup_rule_name(kind: SetupRuleKind) -> String {
    match kind {
        SetupRuleKind::SequentialNumbers => "Sequential Numbers",
        SetupRuleKind::SequentialLetters => "Sequential Letters",
        SetupRuleKind::ListOfLetters => "List of Letters",
        SetupRuleKind::ListOfWords => "List of Words",
    }
    .to_string()
}

#[uniffi::export]
pub fn setup_format_after_add(format: String, rule_number: u32) -> String {
    format_after_add(&format, rule_number as usize)
}

#[uniffi::export]
pub fn setup_format_after_delete(format: String, old_rule_count: u32) -> String {
    format_after_delete(&format, old_rule_count as usize)
}

#[uniffi::export]
pub fn setup_step_after_edit(typed: i32, previous: i32) -> i32 {
    numeric_step_after_edit(typed, previous)
}

/// List of Letters' box: only letters and digits stay.
#[uniffi::export]
pub fn setup_clean_letter_list(text: String) -> String {
    clean_letter_list(&text)
}

#[uniffi::export]
pub fn setup_replace_patterns(old: Vec<String>, patterns: Vec<SetupFindReplace>) -> Vec<String> {
    let pairs: Vec<(String, String)> = patterns.into_iter().map(|p| (p.find, p.replace)).collect();
    replace_patterns(&old, &pairs)
}

#[uniffi::export]
pub fn setup_pattern_row_text(index: u32, find: String, replace: String) -> String {
    pattern_row_text(index as usize, &find, &replace)
}

