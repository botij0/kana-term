//! Length-ladder drill: streak, miss reveal, session stats.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::kana::{canonical_hepburn, glyph, gojuon, input_matches, mora_results};

pub const STREAK_TO_LEVEL: u8 = 10;
pub const MAX_LEVEL: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Hiragana,
    Katakana,
}

impl Script {
    pub fn label(self) -> &'static str {
        match self {
            Script::Hiragana => "hiragana",
            Script::Katakana => "katakana",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Typing,
    RevealMiss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatRow {
    pub glyph: char,
    pub seen: u32,
    pub correct: u32,
}

impl StatRow {
    pub fn accuracy(&self) -> f64 {
        if self.seen == 0 {
            0.0
        } else {
            f64::from(self.correct) / f64::from(self.seen)
        }
    }
}

pub struct Drill {
    rng: StdRng,
    script: Script,
    level: u8,
    streak: u8,
    max_level: u8,
    prompt: String,
    input: String,
    phase: Phase,
    revealed: Option<String>,
    stats: HashMap<char, (u32, u32)>,
}

impl Drill {
    pub fn new(script: Script) -> Self {
        Self::from_rng(script, StdRng::from_entropy())
    }

    pub fn from_seed(script: Script, seed: u64) -> Self {
        Self::from_rng(script, StdRng::seed_from_u64(seed))
    }

    fn from_rng(script: Script, rng: StdRng) -> Self {
        let mut drill = Self {
            rng,
            script,
            level: 1,
            streak: 0,
            max_level: 1,
            prompt: String::new(),
            input: String::new(),
            phase: Phase::Typing,
            revealed: None,
            stats: HashMap::new(),
        };
        drill.roll_prompt();
        drill
    }

    pub fn script(&self) -> Script {
        self.script
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn streak(&self) -> u8 {
        self.streak
    }

    pub fn max_level(&self) -> u8 {
        self.max_level
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn revealed_hepburn(&self) -> Option<&str> {
        self.revealed.as_deref()
    }

    pub fn push_char(&mut self, c: char) {
        if self.phase != Phase::Typing {
            return;
        }
        if c.is_ascii_alphabetic() {
            self.input.push(c.to_ascii_lowercase());
        }
    }

    pub fn backspace(&mut self) {
        if self.phase != Phase::Typing {
            return;
        }
        self.input.pop();
    }

    pub fn submit(&mut self) {
        match self.phase {
            Phase::Typing => self.judge(),
            Phase::RevealMiss => self.drop_and_continue(),
        }
    }

    pub fn stat_rows(&self) -> Vec<StatRow> {
        let mut rows: Vec<StatRow> = self
            .stats
            .iter()
            .map(|(&glyph, &(seen, correct))| StatRow {
                glyph,
                seen,
                correct,
            })
            .collect();
        rows.sort_by(|a, b| {
            a.accuracy()
                .partial_cmp(&b.accuracy())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.seen.cmp(&a.seen))
                .then_with(|| a.glyph.cmp(&b.glyph))
        });
        rows
    }

    pub fn mora_totals(&self) -> (u32, u32) {
        self.stats
            .values()
            .fold((0, 0), |(seen, correct), &(s, c)| (seen + s, correct + c))
    }

    fn judge(&mut self) {
        let hit = input_matches(&self.prompt, &self.input);
        for (glyph, correct) in mora_results(&self.prompt, &self.input) {
            let entry = self.stats.entry(glyph).or_insert((0, 0));
            entry.0 += 1;
            if correct {
                entry.1 += 1;
            }
        }
        if hit {
            self.streak += 1;
            if self.streak >= STREAK_TO_LEVEL {
                if self.level < MAX_LEVEL {
                    self.level += 1;
                    self.max_level = self.max_level.max(self.level);
                }
                self.streak = 0;
            }
            self.roll_prompt();
        } else {
            self.phase = Phase::RevealMiss;
            self.revealed = Some(canonical_hepburn(&self.prompt));
        }
    }

    fn drop_and_continue(&mut self) {
        self.level = self.level.saturating_sub(1).max(1);
        self.streak = 0;
        self.roll_prompt();
    }

    fn roll_prompt(&mut self) {
        let n = self.level as usize;
        let katakana = matches!(self.script, Script::Katakana);
        let mut pool: Vec<_> = gojuon().iter().collect();
        pool.shuffle(&mut self.rng);
        self.prompt = pool
            .into_iter()
            .take(n)
            .map(|k| glyph(k, katakana))
            .collect();
        self.input.clear();
        self.phase = Phase::Typing;
        self.revealed = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_hepburn(drill: &mut Drill) {
        let answer = canonical_hepburn(drill.prompt());
        for c in answer.chars() {
            drill.push_char(c);
        }
    }

    fn correct(drill: &mut Drill) {
        type_hepburn(drill);
        drill.submit();
    }

    #[test]
    fn new_drill_starts_at_level_one() {
        let drill = Drill::from_seed(Script::Hiragana, 1);
        assert_eq!(drill.level(), 1);
        assert_eq!(drill.streak(), 0);
        assert_eq!(drill.prompt().chars().count(), 1);
        assert_eq!(drill.phase(), Phase::Typing);
    }

    #[test]
    fn ten_correct_in_a_row_levels_up() {
        let mut drill = Drill::from_seed(Script::Hiragana, 2);
        for _ in 0..STREAK_TO_LEVEL {
            assert_eq!(drill.level(), 1);
            correct(&mut drill);
        }
        assert_eq!(drill.level(), 2);
        assert_eq!(drill.streak(), 0);
        assert_eq!(drill.prompt().chars().count(), 2);
    }

    #[test]
    fn miss_reveals_hepburn_then_drops_after_enter() {
        let mut drill = Drill::from_seed(Script::Hiragana, 3);
        for _ in 0..STREAK_TO_LEVEL {
            correct(&mut drill);
        }
        assert_eq!(drill.level(), 2);
        drill.push_char('x');
        drill.submit();
        assert_eq!(drill.phase(), Phase::RevealMiss);
        assert_eq!(drill.level(), 2);
        let expected = canonical_hepburn(drill.prompt());
        assert_eq!(drill.revealed_hepburn(), Some(expected.as_str()));
        drill.submit();
        assert_eq!(drill.phase(), Phase::Typing);
        assert_eq!(drill.level(), 1);
        assert_eq!(drill.max_level(), 2);
        assert_eq!(drill.streak(), 0);
        assert_eq!(drill.prompt().chars().count(), 1);
    }

    #[test]
    fn miss_at_level_one_stays_at_level_one() {
        let mut drill = Drill::from_seed(Script::Hiragana, 4);
        drill.push_char('x');
        drill.submit();
        drill.submit();
        assert_eq!(drill.level(), 1);
    }

    #[test]
    fn prompt_has_no_repeats() {
        let mut drill = Drill::from_seed(Script::Hiragana, 5);
        for _ in 0..STREAK_TO_LEVEL {
            correct(&mut drill);
        }
        for _ in 0..STREAK_TO_LEVEL {
            correct(&mut drill);
        }
        assert_eq!(drill.level(), 3);
        let chars: Vec<char> = drill.prompt().chars().collect();
        let mut unique = chars.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), chars.len());
    }

    #[test]
    fn level_caps_at_ten() {
        let mut drill = Drill::from_seed(Script::Hiragana, 6);
        for _ in 0..(MAX_LEVEL - 1) * STREAK_TO_LEVEL {
            correct(&mut drill);
        }
        assert_eq!(drill.level(), MAX_LEVEL);
        for _ in 0..STREAK_TO_LEVEL {
            correct(&mut drill);
        }
        assert_eq!(drill.level(), MAX_LEVEL);
        assert_eq!(drill.prompt().chars().count(), MAX_LEVEL as usize);
    }

    #[test]
    fn stats_credit_correct_morae_on_a_miss() {
        let mut drill = Drill::from_seed(Script::Hiragana, 7);
        for _ in 0..STREAK_TO_LEVEL {
            correct(&mut drill);
        }
        assert_eq!(drill.level(), 2);
        let glyphs: Vec<char> = drill.prompt().chars().collect();
        let first = canonical_hepburn(&glyphs[0].to_string());
        for c in first.chars() {
            drill.push_char(c);
        }
        drill.push_char('q');
        drill.submit();
        let rows = drill.stat_rows();
        let first_row = rows.iter().find(|r| r.glyph == glyphs[0]).unwrap();
        let second_row = rows.iter().find(|r| r.glyph == glyphs[1]).unwrap();
        assert_eq!(first_row.correct, 1);
        assert_eq!(second_row.correct, 0);
        assert!(second_row.accuracy() <= first_row.accuracy());
    }

    #[test]
    fn katakana_prompt_uses_katakana_glyphs() {
        let drill = Drill::from_seed(Script::Katakana, 8);
        let ch = drill.prompt().chars().next().unwrap();
        assert!((ch as u32) >= 0x30A0, "expected katakana, got {ch}");
    }

    #[test]
    fn backspace_edits_input() {
        let mut drill = Drill::from_seed(Script::Hiragana, 9);
        drill.push_char('k');
        drill.push_char('a');
        drill.backspace();
        assert_eq!(drill.input(), "k");
    }
}
