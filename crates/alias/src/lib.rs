//! Footnote Alias, stage 1: identifiers with a fixed format, found by rules and kept only when their check digits are valid
//! (docs/research/ALIAS-DESK-STUDY.md). French first: NIR, SIREN/SIRET, French IBAN and phones; e-mail, cards and IPv4 everywhere.
//! Stage 2, names, addresses and dates, is a model ([`model`]); [`find`] combines both and [`pseudonymise`] replaces the hits.
pub mod model;

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hit {
    /// Character offsets (not bytes), so callers in any language can slice the text.
    pub start: usize,
    pub end: usize,
    pub label: &'static str,
    pub value: String,
}

struct Rule {
    label: &'static str,
    re: Regex,
    valid: fn(&str) -> bool,
}

fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

fn luhn(d: &str) -> bool {
    let sum: u32 = d
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let v = u32::from(b - b'0') * if i % 2 == 1 { 2 } else { 1 };
            if v > 9 { v - 9 } else { v }
        })
        .sum();
    !d.is_empty() && sum.is_multiple_of(10)
}

/// ISO 13616: move the first four characters to the end, letters become 10..35, the number mod 97 must be 1.
fn iban(s: &str) -> bool {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !(15..=34).contains(&s.len()) {
        return false;
    }
    let rotated = s[4..].chars().chain(s[..4].chars());
    let mut rem = 0u32;
    for c in rotated {
        let Some(v) = c.to_digit(36) else {
            return false;
        };
        for d in v.to_string().bytes() {
            rem = (rem * 10 + u32::from(d - b'0')) % 97;
        }
    }
    rem == 1
}

/// French social security number: 13 characters then a key, 97 minus the number mod 97; Corsica 2A and 2B count as 19 and 18.
fn nir(s: &str) -> bool {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() != 15 {
        return false;
    }
    let body = s[..13].replacen("2A", "19", 1).replacen("2B", "18", 1);
    match (body.parse::<u64>(), s[13..].parse::<u64>()) {
        (Ok(n), Ok(key)) => 97 - n % 97 == key,
        _ => false,
    }
}

fn card(s: &str) -> bool {
    let d = digits(s);
    (13..=19).contains(&d.len()) && d.len() != 14 && luhn(&d)
}

fn siren_siret(s: &str) -> bool {
    let d = digits(s);
    matches!(d.len(), 9 | 14) && luhn(&d)
}

fn always(_: &str) -> bool {
    true
}

/// In priority order: a span claimed by an earlier rule is not offered to a later one (an IBAN holds digit runs a card rule
/// would also accept).
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let rule = |label, pattern: &str, valid| Rule {
        label,
        re: Regex::new(pattern).expect("valid pattern"),
        valid,
    };
    vec![
        rule(
            "EMAIL",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b",
            always,
        ),
        rule(
            "IBAN",
            r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,3})?\b",
            iban,
        ),
        rule(
            "NIR",
            r"\b[12] ?\d{2} ?\d{2} ?(?:\d{2}|2[AB]) ?\d{3} ?\d{3} ?\d{2}\b",
            nir,
        ),
        rule("SIRET", r"\b\d{3} ?\d{3} ?\d{3} ?\d{5}\b", siren_siret),
        rule("CARD", r"\b\d(?:[ -]?\d){12,18}\b", card),
        rule("SIREN", r"\b\d{3} ?\d{3} ?\d{3}\b", siren_siret),
        rule(
            "IP",
            r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b",
            always,
        ),
        // French numbers (0X or +33 X then four pairs), then international and UK/US-style groupings.
        rule("PHONE", r"(?:\+33 ?|\b0)[1-9](?:[ .-]?\d{2}){4}\b", always),
        rule(
            "PHONE",
            r"\+\d{1,3}(?:[ .-]?\d{1,4}){2,5}\b|\(\d{3}\) ?\d{3,4}-\d{4}\b|\b0\d{4} ?\d{6}\b",
            always,
        ),
    ]
});

/// Every identifier in `text`, in reading order.
pub fn scan(text: &str) -> Vec<Hit> {
    let mut claimed: Vec<(usize, usize)> = Vec::new();
    let mut hits = Vec::new();
    for rule in RULES.iter() {
        for m in rule.re.find_iter(text) {
            let (s, e) = (m.start(), m.end());
            if claimed.iter().any(|&(a, b)| s < b && a < e) || !(rule.valid)(m.as_str()) {
                continue;
            }
            claimed.push((s, e));
            hits.push((s, e, rule.label));
        }
    }
    hits.sort_unstable();
    hits.into_iter()
        .map(|(s, e, label)| Hit {
            start: text[..s].chars().count(),
            end: text[..e].chars().count(),
            label,
            value: text[s..e].to_string(),
        })
        .collect()
}

/// Rules first, then the model's spans that no rule already claimed (the "alias" detector of ALIAS-EVAL.md).
pub fn find(text: &str, model: Option<&mut model::PrivacyFilter>) -> anyhow::Result<Vec<Hit>> {
    let mut hits = scan(text);
    if let Some(m) = model {
        let extra: Vec<Hit> = m
            .detect(text)?
            .into_iter()
            .filter(|p| !hits.iter().any(|h| p.start < h.end && h.start < p.end))
            .collect();
        hits.extend(extra);
    }
    // A value found once is hidden everywhere in the text, whatever its case: the model can miss "élodie" after "Élodie".
    let low: Vec<char> = text.chars().map(fold).collect();
    let mut repeats = Vec::new();
    for h in &hits {
        let needle: Vec<char> = h.value.chars().map(fold).collect();
        for s in 0..low.len().saturating_sub(needle.len() - 1) {
            let e = s + needle.len();
            let whole = |i: Option<&char>| i.is_none_or(|c| !c.is_alphanumeric());
            if low[s..e] == needle[..]
                && whole(s.checked_sub(1).and_then(|i| low.get(i)))
                && whole(low.get(e))
                && !hits.iter().chain(&repeats).any(|o: &Hit| s < o.end && o.start < e)
            {
                let value = text.chars().skip(s).take(needle.len()).collect();
                repeats.push(Hit { start: s, end: e, label: h.label, value });
            }
        }
    }
    hits.extend(repeats);
    hits.sort_by_key(|h| h.start);
    Ok(hits)
}

/// One character to one character, so offsets survive (`to_lowercase` can return several).
fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

#[derive(Debug, Serialize)]
pub struct Replacement {
    pub pseudonym: String,
    pub label: &'static str,
    pub value: String,
}

/// Replace each hit by a keyed pseudonym such as `[PERSON_3f9a1c]`. The same value (case and spacing ignored) gets the same
/// pseudonym across texts and runs under one key, so an AI can follow who is who; without the key it cannot be reversed.
pub fn pseudonymise(text: &str, hits: &[Hit], key: &[u8]) -> (String, Vec<Replacement>) {
    let chars: Vec<char> = text.chars().collect();
    let (mut out, mut at, mut table) = (String::new(), 0, Vec::<Replacement>::new());
    for h in hits {
        if h.start < at {
            continue;
        }
        let norm: String = h.value.to_lowercase().split_whitespace().collect();
        let mac = footnote_core::audit::hmac_sha256(key, format!("{}|{norm}", h.label).as_bytes());
        let pseudonym = format!("[{}_{}]", h.label, &mac[..6]);
        out.extend(&chars[at..h.start]);
        out.push_str(&pseudonym);
        at = h.end;
        if !table.iter().any(|r| r.pseudonym == pseudonym) {
            table.push(Replacement { pseudonym, label: h.label, value: h.value.clone() });
        }
    }
    out.extend(&chars[at..]);
    (out, table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_found_once_is_hidden_everywhere_whatever_its_case() {
        let text = "IBAN FR76 3000 6000 0112 3456 7890 189, recopié fr76 3000 6000 0112 3456 7890 189, pas xfr76 3000 6000 0112 3456 7890 189";
        let hits = find(text, None).unwrap();
        let values: Vec<&str> = hits.iter().map(|h| h.value.as_str()).collect();
        assert_eq!(values, ["FR76 3000 6000 0112 3456 7890 189", "fr76 3000 6000 0112 3456 7890 189"], "{hits:?}");
        assert!(hits.iter().all(|h| h.label == "IBAN"));
    }

    #[test]
    fn pseudonyms_are_stable_keyed_and_replace_in_place() {
        let text = "Écrire à elodie.roux@example.org puis à ELODIE.ROUX@example.org, tel 06 12 34 56 78.";
        let (out, table) = pseudonymise(text, &scan(text), &[7; 32]);
        assert_eq!(table.len(), 2, "same address in two cases is one person: {table:?}");
        assert!(out.starts_with("Écrire à [EMAIL_") && out.ends_with("].") && !out.contains('@'), "{out}");
        assert_eq!(out.matches(&table[0].pseudonym).count(), 2, "{out}");
        let (other, _) = pseudonymise(text, &scan(text), &[8; 32]);
        assert_ne!(out, other, "another key gives other pseudonyms");
    }

    fn labels(text: &str) -> Vec<(&'static str, String)> {
        scan(text).into_iter().map(|h| (h.label, h.value)).collect()
    }

    #[test]
    fn valid_identifiers_are_found_and_look_alikes_are_not() {
        let text = "Élodie, IBAN FR76 3000 6000 0112 3456 7890 189, NIR 2 84 05 2A 123 456 77, \
                    SIRET 732 829 320 00074, carte 4970 1012 3456 7893, mail elodie.roux@example.org, \
                    tel 06 12 34 56 78, IP 192.0.2.10, colis 1408493579696837, commande CMD-2026-48213.";
        let found = labels(text);
        let want = |l: &str, v: &str| {
            assert!(
                found.iter().any(|(a, b)| *a == l && b == v),
                "{l} {v}: {found:?}"
            )
        };
        want("IBAN", "FR76 3000 6000 0112 3456 7890 189");
        want("SIRET", "732 829 320 00074");
        want("CARD", "4970 1012 3456 7893");
        want("EMAIL", "elodie.roux@example.org");
        want("PHONE", "06 12 34 56 78");
        want("IP", "192.0.2.10");
        assert!(
            !found.iter().any(|(_, v)| v.contains("1408493579696837")),
            "tracking number fails Luhn: {found:?}"
        );
        assert!(
            !found.iter().any(|(_, v)| v.contains("48213")),
            "order number: {found:?}"
        );
    }

    #[test]
    fn check_digits_decide() {
        assert!(iban("FR7630006000011234567890189"));
        assert!(!iban("FR7630006000011234567890188"));
        assert!(luhn("4970101234567893") && !luhn("4970101234567899"));
        let body = 2_840_519_123_456u64; // 2A counts as 19
        let key = 97 - body % 97;
        assert!(nir(&format!("284052A123456{key:02}")));
        assert!(!nir(&format!("284052A123456{:02}", (key + 1) % 97)));
    }

    #[test]
    fn offsets_are_characters_not_bytes() {
        let raw = "Hélène, à joindre : helene.lefevre@example.com";
        let hits = scan(raw);
        let h = hits.iter().find(|h| h.label == "EMAIL").expect("email");
        let text: Vec<char> = raw.chars().collect();
        assert_eq!(text[h.start..h.end].iter().collect::<String>(), h.value);
    }
}
