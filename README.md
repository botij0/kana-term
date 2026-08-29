# kana-term

A terminal drill for the Japanese **gojūon**: you see hiragana or katakana and type the reading in romaji. Strings get longer as you stay accurate.

Hiragana and katakana are separate modes. Nothing is saved; each launch starts at level 1.

## Run

Needs [Rust](https://rustup.rs/) and a terminal font that can draw Japanese (for example Noto Sans CJK).

```bash
cargo run                 # mode menu
cargo run -- hiragana     # skip the menu
cargo run -- katakana
```

## How a session works

Level **N** shows **N** random characters as one string (no repeats in that string). Type the whole reading with **no spaces**, then Enter.

| You | What happens |
|-----|----------------|
| 10 correct in a row | Level goes up by 1 (longest string is 10) |
| Miss | The Hepburn reading is shown; Enter drops you one level (never below 1) |
| Esc or Ctrl-C | Session stats, worst kana first, then Enter or `q` to quit |

Example at level 3: you see `さしか` and type `sashika` (or `sasika`).

Every trial is a **new** random string of the current length. Progress does not carry across launches.

## What counts as correct

Romaji is case-insensitive. Hepburn is the spelling shown on a miss; common Kunrei forms are also accepted:

| Kana | Accepted |
|------|----------|
| し | `shi`, `si` |
| ち | `chi`, `ti` |
| つ | `tsu`, `tu` |
| ふ | `fu`, `hu` |
| を | `wo`, `o` |
| ん | `n`, `nn` (`nn` before a vowel or `y`, so `んあ` is `nna`, not `na`) |

The pool is the modern 46 (including `を` and `ん`, not obsolete `ゐ`/`ゑ`). Dakuten, yōon, mixed scripts, and kanji are out of scope.

## Keys

**Menu:** ↑↓ or `j`/`k`, Enter to start, Esc to leave.

**Drill:** letters to type, Backspace, Enter to submit. After a miss, Enter continues.

**Stats:** Enter or `q` to exit.
