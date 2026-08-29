use std::io;
use std::time::{Duration, Instant};

use kana_term::{Drill, Phase, Script, StatRow, STREAK_TO_LEVEL};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};

const BG: Color = Color::Rgb(16, 20, 30);
const PANEL: Color = Color::Rgb(30, 38, 54);
const CYAN: Color = Color::Rgb(94, 210, 213);
const GOLD: Color = Color::Rgb(245, 214, 122);
const MUTED: Color = Color::Rgb(130, 140, 155);
const RED: Color = Color::Rgb(232, 96, 108);
const GREEN: Color = Color::Rgb(122, 210, 150);

const MENU_ITEMS: [&str; 3] = ["Hiragana", "Katakana", "Quit"];

enum Screen {
    Menu { selected: usize },
    Drill { drill: Box<Drill>, started: Instant },
    Stats(StatsView),
}

struct StatsView {
    script: Script,
    max_level: u8,
    duration: Duration,
    seen: u32,
    correct: u32,
    rows: Vec<StatRow>,
}

pub enum Cli {
    Help,
    Menu,
    Script(Script),
    Error(String),
}

pub fn parse_args(args: &[String]) -> Cli {
    match args {
        [] => Cli::Menu,
        [flag] if matches!(flag.as_str(), "-h" | "--help") => Cli::Help,
        [mode] if matches!(mode.as_str(), "hiragana" | "--hiragana") => {
            Cli::Script(Script::Hiragana)
        }
        [mode] if matches!(mode.as_str(), "katakana" | "--katakana") => {
            Cli::Script(Script::Katakana)
        }
        _ => Cli::Error(format!("unknown arguments: {}\n{HELP}", args.join(" "))),
    }
}

pub const HELP: &str = "\
kana-term — gojūon trainer

Usage:
  kana-term              Open the mode menu
  kana-term hiragana     Start hiragana (skip menu)
  kana-term katakana     Start katakana (skip menu)
  kana-term --help       Show this help
";

pub fn run(terminal: &mut DefaultTerminal, cli: Cli) -> io::Result<()> {
    let mut screen = match cli {
        Cli::Help | Cli::Error(_) => unreachable!("handled in main"),
        Cli::Menu => Screen::Menu { selected: 0 },
        Cli::Script(script) => Screen::Drill {
            drill: Box::new(Drill::new(script)),
            started: Instant::now(),
        },
    };

    loop {
        terminal.draw(|frame| draw(frame, &screen))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if quit_without_handling(&key, &screen) {
            if matches!(screen, Screen::Drill { .. }) {
                screen = to_stats(screen);
            } else {
                return Ok(());
            }
            continue;
        }
        match handle_key(&mut screen, key) {
            Action::Continue => {}
            Action::Exit => return Ok(()),
        }
    }
}

enum Action {
    Continue,
    Exit,
}

fn quit_without_handling(key: &KeyEvent, screen: &Screen) -> bool {
    let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
    let esc = key.code == KeyCode::Esc;
    if !ctrl_c && !esc {
        return false;
    }
    !matches!(screen, Screen::Stats(_))
}

fn handle_key(screen: &mut Screen, key: KeyEvent) -> Action {
    match screen {
        Screen::Menu { selected } => match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                *selected = selected.checked_sub(1).unwrap_or(MENU_ITEMS.len() - 1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                *selected = (*selected + 1) % MENU_ITEMS.len();
            }
            KeyCode::Enter => match *selected {
                0 => {
                    *screen = Screen::Drill {
                        drill: Box::new(Drill::new(Script::Hiragana)),
                        started: Instant::now(),
                    };
                }
                1 => {
                    *screen = Screen::Drill {
                        drill: Box::new(Drill::new(Script::Katakana)),
                        started: Instant::now(),
                    };
                }
                _ => return Action::Exit,
            },
            _ => {}
        },
        Screen::Drill { drill, .. } => match key.code {
            KeyCode::Enter => drill.submit(),
            KeyCode::Backspace => drill.backspace(),
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                drill.push_char(c);
            }
            _ => {}
        },
        Screen::Stats(_) => match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => return Action::Exit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Action::Exit;
            }
            _ => {}
        },
    }
    Action::Continue
}

fn to_stats(screen: Screen) -> Screen {
    let Screen::Drill { drill, started } = screen else {
        return screen;
    };
    let (seen, correct) = drill.mora_totals();
    Screen::Stats(StatsView {
        script: drill.script(),
        max_level: drill.max_level(),
        duration: started.elapsed(),
        seen,
        correct,
        rows: drill.stat_rows(),
    })
}

fn paint_bg(frame: &mut Frame) {
    frame.render_widget(Block::new().style(Style::default().bg(BG)), frame.area());
}

fn inset(area: Rect, percent: u16) -> Rect {
    Layout::horizontal([
        Constraint::Percentage(percent),
        Constraint::Percentage(100 - percent * 2),
        Constraint::Percentage(percent),
    ])
    .split(area)[1]
}

fn spaced_kana(prompt: &str) -> String {
    prompt
        .chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("   ")
}

fn streak_bar(streak: u8) -> String {
    let filled = streak as usize;
    let empty = STREAK_TO_LEVEL as usize - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn vcenter(text: String, height: u16, style: Style) -> Paragraph<'static> {
    let pad = height.saturating_sub(1) / 2;
    let mut lines = vec![Line::from(""); pad as usize];
    lines.push(Line::from(Span::styled(text, style)));
    Paragraph::new(lines).alignment(Alignment::Center)
}

fn card(fg: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(fg))
        .style(Style::default().bg(PANEL))
        .padding(Padding::uniform(1))
}

fn draw(frame: &mut Frame, screen: &Screen) {
    paint_bg(frame);
    match screen {
        Screen::Menu { selected } => draw_menu(frame, *selected),
        Screen::Drill { drill, .. } => draw_drill(frame, drill),
        Screen::Stats(stats) => draw_stats(frame, stats),
    }
}

fn draw_menu(frame: &mut Frame, selected: usize) {
    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(7),
        Constraint::Length(2),
    ])
    .split(frame.area());

    let title = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "kana-term",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "train gojūon in the terminal",
            Style::default().fg(MUTED),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    let menu_area = inset(chunks[1], 18);
    let block = card(CYAN);
    let inner = block.inner(menu_area);
    frame.render_widget(block, menu_area);

    let pad = inner.height.saturating_sub(MENU_ITEMS.len() as u16) / 2;
    let mut lines = vec![Line::from(""); pad as usize];
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        if i == selected {
            lines.push(Line::from(Span::styled(
                format!("  ▸ {item}  "),
                Style::default()
                    .fg(BG)
                    .bg(GOLD)
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("    {item}  "),
                Style::default().fg(MUTED),
            )));
        }
    }
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);

    frame.render_widget(
        Paragraph::new("↑↓ move   Enter select   Esc quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        chunks[2],
    );
}

fn draw_drill(frame: &mut Frame, drill: &Drill) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(9),
        Constraint::Length(5),
        Constraint::Length(2),
    ])
    .split(frame.area());

    let miss = drill.phase() == Phase::RevealMiss;
    let accent = if miss { RED } else { CYAN };
    let kana_fg = if miss { RED } else { GOLD };
    let bar_fg = if drill.streak() >= 7 { GREEN } else { GOLD };

    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            drill.script().label(),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   level ", Style::default().fg(MUTED)),
        Span::styled(
            drill.level().to_string(),
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(streak_bar(drill.streak()), Style::default().fg(bar_fg)),
        Span::styled(
            format!("  {}/{}", drill.streak(), STREAK_TO_LEVEL),
            Style::default().fg(MUTED),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header).style(Style::default().bg(BG)),
        chunks[0],
    );

    let kana_area = inset(chunks[1], 10);
    let block = card(accent);
    let inner = block.inner(kana_area);
    frame.render_widget(block, kana_area);
    frame.render_widget(
        vcenter(
            spaced_kana(drill.prompt()),
            inner.height,
            Style::default().fg(kana_fg).add_modifier(Modifier::BOLD),
        ),
        inner,
    );

    let input_area = inset(chunks[2], 18);
    let input_block = card(if miss { RED } else { GOLD });
    let input_inner = input_block.inner(input_area);
    frame.render_widget(input_block, input_area);
    let typed = match drill.phase() {
        Phase::Typing => format!("{}█", drill.input()),
        Phase::RevealMiss => drill.input().to_string(),
    };
    frame.render_widget(
        vcenter(
            typed,
            input_inner.height,
            Style::default().fg(if miss { RED } else { GOLD }),
        ),
        input_inner,
    );

    let hint = match drill.phase() {
        Phase::Typing => Paragraph::new("type the reading, then Enter. Esc quits.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(BG)),
        Phase::RevealMiss => {
            let expected = drill.revealed_hepburn().unwrap_or("");
            Paragraph::new(Line::from(vec![
                Span::styled("expected  ", Style::default().fg(MUTED)),
                Span::styled(
                    expected.to_string(),
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                ),
                Span::styled("    Enter to continue", Style::default().fg(MUTED)),
            ]))
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG))
        }
    };
    frame.render_widget(hint, chunks[3]);
}

fn draw_stats(frame: &mut Frame, stats: &StatsView) {
    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(frame.area());

    let accuracy = if stats.seen == 0 {
        "—".to_string()
    } else {
        format!(
            "{:.0}%",
            100.0 * f64::from(stats.correct) / f64::from(stats.seen)
        )
    };
    let acc_color = if stats.seen == 0 {
        MUTED
    } else if stats.correct * 100 / stats.seen >= 80 {
        GREEN
    } else if stats.correct * 100 / stats.seen >= 50 {
        GOLD
    } else {
        RED
    };

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "session",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(stats.script.label(), Style::default().fg(CYAN)),
            Span::styled("   max level ", Style::default().fg(MUTED)),
            Span::styled(
                stats.max_level.to_string(),
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   {}   accuracy ", fmt_duration(stats.duration)),
                Style::default().fg(MUTED),
            ),
            Span::styled(
                accuracy,
                Style::default().fg(acc_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "kana   seen  correct  accuracy",
            Style::default().fg(MUTED),
        )),
    ])
    .style(Style::default().bg(BG));
    frame.render_widget(header, chunks[0]);

    let table_area = inset(chunks[1], 8);
    let mut lines: Vec<Line> = Vec::new();
    if stats.rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "no trials this session",
            Style::default().fg(MUTED),
        )));
    } else {
        for row in &stats.rows {
            let pct = row.accuracy() * 100.0;
            let color = if pct >= 80.0 {
                GREEN
            } else if pct >= 50.0 {
                GOLD
            } else {
                RED
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        " {}     {:>4}     {:>4}     ",
                        row.glyph, row.seen, row.correct
                    ),
                    Style::default().fg(CYAN),
                ),
                Span::styled(
                    format!("{pct:>3.0}%"),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(BG)),
        table_area,
    );
    frame.render_widget(
        Paragraph::new("Enter or q to exit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(BG)),
        chunks[2],
    );
}

fn fmt_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args_opens_menu() {
        assert!(matches!(parse_args(&[]), Cli::Menu));
    }

    #[test]
    fn hiragana_flag_skips_menu() {
        assert!(matches!(
            parse_args(&["hiragana".into()]),
            Cli::Script(Script::Hiragana)
        ));
        assert!(matches!(
            parse_args(&["--hiragana".into()]),
            Cli::Script(Script::Hiragana)
        ));
    }
}
