mod big_kana;
mod ui;

use std::io::{self, Write};
use std::process::ExitCode;

use ui::{parse_args, run, Cli, HELP};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Cli::Help => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Cli::Error(msg) => {
            eprintln!("{msg}");
            ExitCode::from(2)
        }
        cli => match run_tui(cli) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::from(1)
            }
        },
    }
}

fn run_tui(cli: Cli) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, cli);
    ratatui::restore();
    io::stdout().flush()?;
    result
}
