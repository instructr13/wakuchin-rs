use std::io::stderr;
use std::panic::{self, PanicInfo};
use std::path::PathBuf;
use std::process;

use anyhow::anyhow;
use clap::Parser;
use clap_serde_derive::ClapSerde;
use console::Term;
use crossterm::{cursor, execute, style::Print};
use dialoguer::{theme::ColorfulTheme, Input};
use regex::Regex;

use crate::config::{load_config, Config};
use crate::error::Result;

use shadow_rs::shadow;

shadow!(build);

#[derive(Parser)]
#[command(author, version, about, long_about = "A next generation wakuchin researcher software written in Rust
P2P-Develop

Wakuchin will generate shuffled \"わくちん\" characters and check if they match a given regex.
If they do, it will print the result to stdout.", long_version = build::CLAP_LONG_VERSION, after_long_help = "For more information, see GitHub repository: https://github.com/P2P-Develop/wakuchin-rs")]
struct Args {
  /// Config file path, can be json, yaml, or toml, detected by extension
  #[arg(value_name = "FILE")]
  config_path: Option<PathBuf>,

  /// Rest of arguments
  #[command(flatten)]
  config: <Config as ClapSerde>::Opt,
}

pub(crate) struct App {
  args: Args,
  pub(crate) config: Config,
  interactive: bool,
}

impl App {
  pub(crate) fn new() -> Self {
    let interactive =
      atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stderr);

    App {
      args: Args::parse(),
      config: Config::default(),
      interactive,
    }
  }

  fn check_interactive(&self) {
    if !self.interactive {
      eprintln!("error: Cannot prompt in non-interactive mode (hint: pipe stdin/stderr to tty or fill in the missing arguments)");

      process::exit(1);
    }
  }

  fn prompt_tries(&self, term: &Term) -> Result<usize> {
    self.check_interactive();

    Input::<usize>::with_theme(&ColorfulTheme::default())
      .with_prompt("How many tries:")
      .interact_on(term)
      .map_err(|e| e.into())
  }

  fn prompt_times(&self, term: &Term) -> Result<usize> {
    self.check_interactive();

    Input::<usize>::with_theme(&ColorfulTheme::default())
      .with_prompt("Wakuchins times:")
      .interact_on(term)
      .map_err(|e| e.into())
  }

  fn prompt_regex(&self, term: &Term) -> Result<Regex> {
    self.check_interactive();

    let regex = Input::<String>::with_theme(&ColorfulTheme::default())
      .with_prompt("Regex to detect hits:")
      .validate_with(|s: &String| {
        if s.is_empty() {
          Err("Regex is empty")
        } else if Regex::new(s).is_err() {
          Err("Regex is invalid")
        } else {
          Ok(())
        }
      })
      .interact_text_on(term)?;

    Regex::new(&regex).map_err(|e| anyhow!(e).into())
  }

  pub(crate) fn set_panic_hook(
  ) -> Box<dyn Fn(&PanicInfo) + Send + Sync + 'static> {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(|panic_info| {
      execute!(
        stderr(),
        cursor::Show,
        Print("\n"),
        cursor::MoveUp(1),
        cursor::MoveLeft(u16::MAX),
        Print("wakuchin has panicked.\n"),
        Print("Please report this to the author.\n"),
        Print(format!("{}", panic_info)),
        cursor::MoveLeft(u16::MAX),
      )
      .unwrap();

      process::exit(1);
    }));

    default_hook
  }

  pub(crate) async fn setup_config(&mut self) -> Result<()> {
    let mut config = if let Some(config_path) = &self.args.config_path {
      load_config(config_path.as_path())
        .await?
        .merge(&mut self.args.config)
    } else {
      Config::from(&mut self.args.config)
    };

    let term = Term::buffered_stderr();

    if config.tries == 0 {
      config.tries = self.prompt_tries(&term)?;
    }

    if config.times == 0 {
      config.times = self.prompt_times(&term)?;
    }

    if config.regex.as_str() == "%default%" {
      config.regex = self.prompt_regex(&term)?;
    }

    self.config = config;

    Ok(())
  }
}
