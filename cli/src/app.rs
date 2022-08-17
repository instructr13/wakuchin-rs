use std::{io::Error, path::Path, time::Duration};

use clap::Parser;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Input};
use regex::Regex;
use serde::Deserialize;
use serde_with::{serde_as, DurationMilliSeconds};

use wakuchin::result::ResultOutputFormat;

use crate::config::load_config;

type AnyhowResult<T> = anyhow::Result<T, Box<dyn std::error::Error>>;

fn value_parser_format(s: &str) -> Result<ResultOutputFormat, String> {
  match s {
    "text" => Ok(ResultOutputFormat::Text),
    "json" => Ok(ResultOutputFormat::Json),
    _ => Err(format!("Unknown format: {}", s)),
  }
}

fn default_duration() -> Duration {
  Duration::from_millis(300)
}

fn parse_duration(arg: &str) -> Result<Duration, std::num::ParseIntError> {
  let seconds = arg.parse()?;
  Ok(Duration::from_millis(seconds))
}

#[serde_as]
#[derive(Clone, Debug, Parser, Deserialize)]
#[clap(author, version, about, long_about = None)]
pub(crate) struct Config {
  #[clap(
    short = 'i',
    long,
    value_parser,
    value_name = "N",
    help = "Number of tries"
  )]
  pub(crate) tries: Option<usize>,

  #[clap(
    short,
    long,
    value_parser,
    value_name = "N",
    help = "Wakuchin times n"
  )]
  pub(crate) times: Option<usize>,

  #[serde(default)]
  #[serde(with = "serde_regex")]
  #[clap(short, long, value_parser, help = "Regex to detect hits")]
  pub(crate) regex: Option<Regex>,

  #[serde(rename(deserialize = "output"))]
  #[clap(short = 'f', long = "format", value_parser = value_parser_format, value_name = "text|json", help = "Output format")]
  pub(crate) out: Option<ResultOutputFormat>,

  #[clap(
    value_name = "config",
    help = "Config file path, can be json, yaml, and toml, detected by extension"
  )]
  pub(crate) config: Option<String>,

  #[serde(default = "default_duration")]
  #[serde_as(as = "DurationMilliSeconds<u64>")]
  #[clap(
    short = 'd',
    long = "interval",
    value_name = "DURATION",
    help = "Progress refresh interval, in milliseconds",
    default_value = "300",
    parse(try_from_str = parse_duration)
  )]
  pub(crate) interval: Duration,

  #[cfg(not(feature = "sequential"))]
  #[serde(default)]
  #[clap(
    short = 'w',
    long = "workers",
    value_name = "N",
    help = "Number of workers, 0 means number of logical CPUs",
    default_value = "0"
  )]
  pub(crate) workers: usize,
}

pub(crate) struct App {
  pub(crate) args: Config,
  interactive: bool,
}

impl App {
  pub(crate) fn new() -> AnyhowResult<Self> {
    let interactive =
      atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stderr);

    Ok(App {
      args: Config::parse(),
      interactive,
    })
  }

  fn unwrap_or_else_fn<T>(error: Error) -> T {
    panic!("{}", error);
  }

  fn check_interactive(&self) {
    if !self.interactive {
      panic!("Cannot prompt in non-interactive mode (hint: pipe stdin/stderr to tty or fill in the missing arguments)");
    }
  }

  fn prompt_tries(&self, term: &Term) -> usize {
    self.check_interactive();

    let tries = Input::<usize>::with_theme(&ColorfulTheme::default())
      .with_prompt("How many tries:")
      .interact_on(term);

    tries.unwrap_or_else(Self::unwrap_or_else_fn)
  }

  fn prompt_times(&self, term: &Term) -> usize {
    self.check_interactive();

    let times = Input::<usize>::with_theme(&ColorfulTheme::default())
      .with_prompt("Wakuchins times:")
      .interact_on(term);

    times.unwrap_or_else(Self::unwrap_or_else_fn)
  }

  fn prompt_regex(&self, term: &Term) -> Regex {
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
      .interact_text_on(term)
      .unwrap_or_else(Self::unwrap_or_else_fn);

    Regex::new(&regex).expect("regular expression check has bypassed")
  }

  pub(crate) async fn prompt(&mut self) -> AnyhowResult<Config> {
    let args_config_ref = self.args.config.as_ref();

    if args_config_ref.unwrap_or(&"".to_string()) != "" {
      let config = load_config(Path::new(&args_config_ref.unwrap())).await?;

      self.args.tries = self.args.tries.or(config.tries);
      self.args.times = self.args.times.or(config.times);
      self.args.regex =
        self.args.regex.as_ref().or(config.regex.as_ref()).cloned();
      self.args.out = self.args.out.as_ref().or(config.out.as_ref()).cloned();
    }

    let term = Term::buffered_stderr();

    if self.args.tries.is_none() {
      self.args.tries = Some(self.prompt_tries(&term));
    }

    if self.args.times.is_none() {
      self.args.times = Some(self.prompt_times(&term));
    }

    if self.args.regex.is_none() {
      self.args.regex = Some(self.prompt_regex(&term));
    }

    self.args.out = self
      .args
      .out
      .as_ref()
      .or(Some(&ResultOutputFormat::Text))
      .cloned();

    Ok(self.args.clone())
  }
}
