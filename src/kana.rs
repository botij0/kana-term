//! Gojūon inventory, romanization, and mora-level comparison.

#[derive(Clone, Copy, Debug)]
pub(crate) struct Kana {
    hiragana: char,
    katakana: char,
    hepburn: &'static str,
    alts: &'static [&'static str],
}

const GOJUON: [Kana; 46] = [
    kana('あ', 'ア', "a", &[]),
    kana('い', 'イ', "i", &[]),
    kana('う', 'ウ', "u", &[]),
    kana('え', 'エ', "e", &[]),
    kana('お', 'オ', "o", &[]),
    kana('か', 'カ', "ka", &[]),
    kana('き', 'キ', "ki", &[]),
    kana('く', 'ク', "ku", &[]),
    kana('け', 'ケ', "ke", &[]),
    kana('こ', 'コ', "ko", &[]),
    kana('さ', 'サ', "sa", &[]),
    kana('し', 'シ', "shi", &["si"]),
    kana('す', 'ス', "su", &[]),
    kana('せ', 'セ', "se", &[]),
    kana('そ', 'ソ', "so", &[]),
    kana('た', 'タ', "ta", &[]),
    kana('ち', 'チ', "chi", &["ti"]),
    kana('つ', 'ツ', "tsu", &["tu"]),
    kana('て', 'テ', "te", &[]),
    kana('と', 'ト', "to", &[]),
    kana('な', 'ナ', "na", &[]),
    kana('に', 'ニ', "ni", &[]),
    kana('ぬ', 'ヌ', "nu", &[]),
    kana('ね', 'ネ', "ne", &[]),
    kana('の', 'ノ', "no", &[]),
    kana('は', 'ハ', "ha", &[]),
    kana('ひ', 'ヒ', "hi", &[]),
    kana('ふ', 'フ', "fu", &["hu"]),
    kana('へ', 'ヘ', "he", &[]),
    kana('ほ', 'ホ', "ho", &[]),
    kana('ま', 'マ', "ma", &[]),
    kana('み', 'ミ', "mi", &[]),
    kana('む', 'ム', "mu", &[]),
    kana('め', 'メ', "me", &[]),
    kana('も', 'モ', "mo", &[]),
    kana('や', 'ヤ', "ya", &[]),
    kana('ゆ', 'ユ', "yu", &[]),
    kana('よ', 'ヨ', "yo", &[]),
    kana('ら', 'ラ', "ra", &[]),
    kana('り', 'リ', "ri", &[]),
    kana('る', 'ル', "ru", &[]),
    kana('れ', 'レ', "re", &[]),
    kana('ろ', 'ロ', "ro", &[]),
    kana('わ', 'ワ', "wa", &[]),
    kana('を', 'ヲ', "wo", &["o"]),
    kana('ん', 'ン', "n", &["nn"]),
];

const fn kana(
    hiragana: char,
    katakana: char,
    hepburn: &'static str,
    alts: &'static [&'static str],
) -> Kana {
    Kana {
        hiragana,
        katakana,
        hepburn,
        alts,
    }
}

pub(crate) fn gojuon() -> &'static [Kana] {
    &GOJUON
}

pub(crate) fn glyph(kana: &Kana, katakana: bool) -> char {
    if katakana {
        kana.katakana
    } else {
        kana.hiragana
    }
}

fn lookup(ch: char) -> Option<&'static Kana> {
    GOJUON.iter().find(|k| k.hiragana == ch || k.katakana == ch)
}

fn parse_prompt(kana: &str) -> Vec<&'static Kana> {
    kana.chars()
        .map(|ch| lookup(ch).expect("prompt contains only gojūon"))
        .collect()
}

fn starts_with_vowel_or_y(reading: &str) -> bool {
    matches!(
        reading.as_bytes().first(),
        Some(b'a' | b'i' | b'u' | b'e' | b'o' | b'y')
    )
}

fn all_readings(kana: &Kana) -> impl Iterator<Item = &'static str> {
    std::iter::once(kana.hepburn).chain(kana.alts.iter().copied())
}

fn next_starts_with_vowel_or_y(next: Option<&Kana>) -> bool {
    next.is_some_and(|n| all_readings(n).any(starts_with_vowel_or_y))
}

fn is_n(kana: &Kana) -> bool {
    kana.hiragana == 'ん'
}

/// Readings that may be used for this kana given the following mora.
fn readings_for(kana: &Kana, next: Option<&Kana>) -> Vec<&'static str> {
    let mut readings: Vec<&'static str> = all_readings(kana).collect();
    if is_n(kana) && next_starts_with_vowel_or_y(next) {
        readings.retain(|r| *r != "n");
    }
    readings.sort_by_key(|r| std::cmp::Reverse(r.len()));
    readings
}

fn normalize(input: &str) -> String {
    input.to_ascii_lowercase()
}

fn match_seq(kana: &[&Kana], input: &str) -> bool {
    if kana.is_empty() {
        return input.is_empty();
    }
    let next = kana.get(1).copied();
    for reading in readings_for(kana[0], next) {
        if let Some(rest) = input.strip_prefix(reading) {
            if match_seq(&kana[1..], rest) {
                return true;
            }
        }
    }
    false
}

/// True when `romaji` is an accepted reading of the kana string.
pub fn input_matches(kana: &str, romaji: &str) -> bool {
    let prompt = parse_prompt(kana);
    match_seq(&prompt, &normalize(romaji))
}

/// Hepburn spelling shown on a miss. `ん` before a vowel or `y` uses `nn`.
pub fn canonical_hepburn(kana: &str) -> String {
    let prompt = parse_prompt(kana);
    let mut out = String::new();
    for (i, k) in prompt.iter().enumerate() {
        let next = prompt.get(i + 1).copied();
        let readings = readings_for(k, next);
        if readings.contains(&k.hepburn) {
            out.push_str(k.hepburn);
        } else {
            out.push_str(readings[0]);
        }
    }
    out
}

fn consume_any_mora(input: &mut &str) {
    if input.is_empty() {
        return;
    }
    let mut all: Vec<&'static str> = GOJUON.iter().flat_map(all_readings).collect();
    all.sort_by_key(|r| std::cmp::Reverse(r.len()));
    all.dedup();
    for reading in all {
        if let Some(rest) = input.strip_prefix(reading) {
            *input = rest;
            return;
        }
    }
    let ch = input.chars().next().expect("non-empty");
    *input = &input[ch.len_utf8()..];
}

/// Pair each expected kana with whether the typed romaji matched that mora.
pub fn mora_results(kana: &str, romaji: &str) -> Vec<(char, bool)> {
    let prompt = parse_prompt(kana);
    let normalized = normalize(romaji);
    let mut rest = normalized.as_str();
    let mut out = Vec::with_capacity(prompt.len());
    for (i, k) in prompt.iter().enumerate() {
        let shown = kana.chars().nth(i).expect("kana length");
        let next = prompt.get(i + 1).copied();
        if let Some(reading) = readings_for(k, next)
            .into_iter()
            .find(|r| rest.starts_with(*r))
        {
            rest = &rest[reading.len()..];
            out.push((shown, true));
        } else if rest.is_empty() {
            out.push((shown, false));
        } else {
            consume_any_mora(&mut rest);
            out.push((shown, false));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gojuon_has_46_characters() {
        assert_eq!(GOJUON.len(), 46);
    }

    #[test]
    fn ka_matches_ka() {
        assert!(input_matches("か", "ka"));
        assert!(input_matches("カ", "ka"));
        assert!(!input_matches("か", "ki"));
    }

    #[test]
    fn shi_accepts_kunrei() {
        assert!(input_matches("し", "shi"));
        assert!(input_matches("し", "si"));
        assert!(input_matches("し", "SHI"));
    }

    #[test]
    fn concatenated_romaji_has_no_spaces() {
        assert!(input_matches("さしか", "sashika"));
        assert!(input_matches("さしか", "sasika"));
        assert!(!input_matches("さしか", "sa shi ka"));
    }

    #[test]
    fn n_before_vowel_requires_nn() {
        assert!(input_matches("ん", "n"));
        assert!(input_matches("ん", "nn"));
        assert!(input_matches("んあ", "nna"));
        assert!(!input_matches("んあ", "na"));
        assert!(input_matches("な", "na"));
        assert!(input_matches("んや", "nnya"));
        assert!(!input_matches("んや", "nya"));
    }

    #[test]
    fn common_kunrei_and_hepburn_alts() {
        assert!(input_matches("ち", "chi"));
        assert!(input_matches("ち", "ti"));
        assert!(input_matches("つ", "tsu"));
        assert!(input_matches("つ", "tu"));
        assert!(input_matches("ふ", "fu"));
        assert!(input_matches("ふ", "hu"));
        assert!(input_matches("シ", "si"));
    }

    #[test]
    fn wo_accepts_o() {
        assert!(input_matches("を", "wo"));
        assert!(input_matches("を", "o"));
    }

    #[test]
    fn canonical_hepburn_is_hepburn_not_kunrei() {
        assert_eq!(canonical_hepburn("し"), "shi");
        assert_eq!(canonical_hepburn("さしか"), "sashika");
        assert_eq!(canonical_hepburn("んあ"), "nna");
    }

    #[test]
    fn mora_diff_credits_correct_syllables_on_a_miss() {
        let result = mora_results("さしか", "sachika");
        assert_eq!(result, vec![('さ', true), ('し', false), ('か', true)]);
    }

    #[test]
    fn mora_diff_marks_unmatched_when_input_is_short() {
        let result = mora_results("さしか", "saka");
        assert_eq!(result, vec![('さ', true), ('し', false), ('か', false)]);
    }

    #[test]
    fn mora_diff_empty_input_is_all_wrong() {
        assert_eq!(mora_results("か", ""), vec![('か', false)]);
    }
}
