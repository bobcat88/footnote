//! Footnote Alias, stage 1: identifiers with a fixed format, found by rules and kept only when their check digits are valid
//! (docs/research/ALIAS-DESK-STUDY.md). French first: NIR, SIREN/SIRET, French IBAN and phones; e-mail, cards and IPv4 everywhere.
//! Names and addresses need a model (stage 2); nothing here guesses them.
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

#[cfg(test)]
mod tests {
    use super::*;

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
