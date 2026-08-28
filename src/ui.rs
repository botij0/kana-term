use std::io;
use std::time::{Duration, Instant};

use kana_term::{Drill, Phase, Script, StatRow, STREAK_TO_LEVEL};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};

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

fn draw(frame: &mut Frame, screen: &Screen) {
    match screen {
        Screen::Menu { selected } => draw_menu(frame, *selected),
        Screen::Drill { drill, .. } => draw_drill(frame, drill),
        Screen::Stats(stats) => draw_stats(frame, stats),
    }
}

fn draw_menu(frame: &mut Frame, selected: usize) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            "kana-term",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("train gojūon in the terminal"),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    let lines: Vec<Line> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if i == selected {
                Line::from(Span::styled(
                    format!("> {item}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(format!("  {item}"))
            }
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new("↑↓ move   Enter select   Esc quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

fn draw_drill(frame: &mut Frame, drill: &Drill) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(area);

    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(drill.script().label(), Style::default().fg(Color::Cyan)),
        Span::raw("   level "),
        Span::styled(
            drill.level().to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::raw(format!("{}/{}", drill.streak(), STREAK_TO_LEVEL)),
    ]);
    frame.render_widget(Paragraph::new(header), chunks[0]);

    let prompt = Paragraph::new(drill.prompt())
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(prompt, chunks[1]);

    let input = match drill.phase() {
        Phase::Typing => format!("{}█", drill.input()),
        Phase::RevealMiss => drill.input().to_string(),
    };
    frame.render_widget(
        Paragraph::new(input)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow)),
        chunks[2],
    );

    let hint = match drill.phase() {
        Phase::Typing => Paragraph::new("type the reading, then Enter. Esc quits.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        Phase::RevealMiss => {
            let expected = drill.revealed_hepburn().unwrap_or("");
            Paragraph::new(vec![
                Line::from(Span::styled(
                    format!("expected: {expected}"),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "Enter to continue",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .alignment(Alignment::Center)
        }
    };
    frame.render_widget(hint, chunks[3]);
}

fn draw_stats(frame: &mut Frame, stats: &StatsView) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);

    let accuracy = if stats.seen == 0 {
        "—".to_string()
    } else {
        format!(
            "{:.0}%",
            100.0 * f64::from(stats.correct) / f64::from(stats.seen)
        )
    };
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "session",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{}   max level {}   {}   accuracy {}",
            stats.script.label(),
            stats.max_level,
            fmt_duration(stats.duration),
            accuracy
        )),
        Line::from(""),
        Line::from("kana   seen  correct  accuracy"),
    ]);
    frame.render_widget(header, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();
    if stats.rows.is_empty() {
        lines.push(Line::from("no trials this session"));
    } else {
        for row in &stats.rows {
            lines.push(Line::from(format!(
                " {}     {:>4}     {:>4}     {:>3.0}%",
                row.glyph,
                row.seen,
                row.correct,
                row.accuracy() * 100.0
            )));
        }
    }
    frame.render_widget(Paragraph::new(lines), chunks[1]);
    frame.render_widget(
        Paragraph::new("Enter or q to exit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
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
