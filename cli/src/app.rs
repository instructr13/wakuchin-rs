use std::io::stderr;
use std::panic::{self, PanicInfo};
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use console::Term;
use crossterm::{cursor, execute, style::Print};
use dialoguer::{theme::ColorfulTheme, Input};
use humantime::DurationError;
use regex::Regex;
use serde::Deserialize;

use wakuchin::result::ResultOutputFormat;

use crate::config::load_config;
use crate::error::Result;
use crate::handlers::HandlerKind;

use shadow_rs::shadow;

shadow!(build);

fn default_duration() -> Duration {
  Duration::from_millis(300)
}

fn parse_duration(
  duration: &str,
) -> std::result::Result<Duration, DurationError> {
  Ok(duration.parse::<humantime::Duration>()?.into())
}

#[derive(
  Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize,
)]
pub(crate) enum InternalResultOutputFormat {
  #[serde(rename = "text")]
  Text,
  #[serde(rename = "json")]
  Json,
}

impl Default for InternalResultOutputFormat {
  fn default() -> Self {
    Self::Text
  }
}

impl From<InternalResultOutputFormat> for ResultOutputFormat {
  fn from(format: InternalResultOutputFormat) -> Self {
    match format {
      InternalResultOutputFormat::Text => Self::Text,
      InternalResultOutputFormat::Json => Self::Json,
    }
  }
}

#[derive(Clone, Debug, Parser, Deserialize)]
#[command(author, version, about, long_about = None, long_version = build::CLAP_LONG_VERSION)]
pub(crate) struct Config {
  #[arg(short = 'i', long, value_name = "N", help = "Number of tries")]
  pub(crate) tries: Option<usize>,

  #[arg(short, long, value_name = "N", help = "Wakuchin times n")]
  pub(crate) times: Option<usize>,

  #[serde(default)]
  #[serde(with = "serde_regex")]
  #[arg(short, long, help = "Regex to detect hits")]
  pub(crate) regex: Option<Regex>,

  #[serde(default)]
  #[serde(rename(deserialize = "output"))]
  #[arg(
    short = 'f',
    long = "format",
    value_name = "FORMAT",
    value_enum,
    help = "Result output format",
    default_value_t = InternalResultOutputFormat::Text
  )]
  pub(crate) out: InternalResultOutputFormat,

  #[arg(
    value_name = "FILE",
    help = "Config file path, can be json, yaml, and toml, detected by extension"
  )]
  pub(crate) config: Option<PathBuf>,

  #[serde(default = "default_duration")]
  #[serde(with = "humantime_serde")]
  #[arg(
    short = 'd',
    long,
    value_name = "DURATION",
    help = "Progress refresh interval",
    default_value = "300ms",
    value_parser = parse_duration
  )]
  pub(crate) interval: Duration,

  #[cfg(not(feature = "sequential"))]
  #[serde(default)]
  #[arg(
    short,
    long,
    value_name = "N",
    help = "Number of workers, 0 means number of logical CPUs",
    default_value_t = 0
  )]
  pub(crate) workers: usize,

  #[serde(default)]
  #[arg(
    short = 'H',
    long,
    value_enum,
    help = "Progress output handler to use",
    default_value_t = HandlerKind::Console
  )]
  pub(crate) handler: HandlerKind,

  #[serde(default)]
  #[arg(
    long,
    help = "Do not show progress, able to use with --handler=console"
  )]
  pub(crate) no_progress: bool,
}

pub(crate) struct App {
  pub(crate) args: Config,
  interactive: bool,
}

impl App {
  pub(crate) fn new() -> Self {
    let interactive =
      atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stderr);

    App {
      args: Config::parse(),
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
        Print(format!(
          "{:?}",
          panic_info
            .payload()
            .downcast_ref::<&str>()
            .unwrap_or(&"no message")
        )),
        Print(format!("Location: {:?}", panic_info.location())),
        cursor::MoveLeft(u16::MAX),
      )
      .unwrap();

      process::exit(1);
    }));

    default_hook
  }

  pub(crate) async fn prompt(&mut self) -> Result<Config> {
    if let Some(config_path) = &self.args.config {
      let config = load_config(config_path.as_path()).await?;

      self.args.tries = self.args.tries.or(config.tries);
      self.args.times = self.args.times.or(config.times);
      self.args.regex =
        self.args.regex.as_ref().or(config.regex.as_ref()).cloned();
      self.args.out = config.out;
      self.args.interval = config.interval;
      self.args.handler = config.handler;
      self.args.workers = config.workers;
    }

    let term = Term::buffered_stderr();

    if self.args.tries.is_none() {
      self.args.tries = Some(self.prompt_tries(&term)?);
    }

    if self.args.times.is_none() {
      self.args.times = Some(self.prompt_times(&term)?);
    }

    if self.args.regex.is_none() {
      self.args.regex = Some(self.prompt_regex(&term)?);
    }

    Ok(self.args.clone())
  }
}

#[cfg(test)]
mod test {
  use std::time::Duration;

  use humantime::DurationError;

  #[test]
  fn test_parse_duration() -> Result<(), DurationError> {
    use super::parse_duration;

    assert_eq!(parse_duration("1s")?, Duration::from_secs(1));
    assert_eq!(parse_duration("1ms")?, Duration::from_millis(1));
    assert_eq!(parse_duration("1us")?, Duration::from_micros(1));
    assert_eq!(parse_duration("1ns")?, Duration::from_nanos(1));
    assert_eq!(parse_duration("1m")?, Duration::from_secs(60));
    assert_eq!(parse_duration("1h")?, Duration::from_secs(60 * 60));
    assert_eq!(parse_duration("1d")?, Duration::from_secs(60 * 60 * 24));

    Ok(())
  }
}
