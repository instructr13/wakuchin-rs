use std::{path::Path, process, time::Duration};

use clap::Parser;
use inquire::{error::InquireError, CustomType, Text};
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
pub struct Config {
  #[clap(
    short = 'i',
    long,
    value_parser,
    value_name = "N",
    help = "Number of tries"
  )]
  pub tries: Option<usize>,

  #[clap(
    short,
    long,
    value_parser,
    value_name = "N",
    help = "Wakuchin times n"
  )]
  pub times: Option<usize>,

  #[serde(default)]
  #[serde(with = "serde_regex")]
  #[clap(short, long, value_parser, help = "Regex to detect hits")]
  pub regex: Option<Regex>,

  #[serde(rename(deserialize = "output"))]
  #[clap(short = 'f', long = "format", value_parser = value_parser_format, value_name = "text|json", help = "Output format")]
  pub out: Option<ResultOutputFormat>,

  #[clap(
    value_name = "config",
    help = "Config file path, can be json, yaml, and toml, detected by extension"
  )]
  pub config: Option<String>,

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
  pub interval: Duration,

  #[cfg(not(feature = "sequential"))]
  #[clap(
    short = 'w',
    long = "workers",
    value_name = "N",
    help = "Number of workers, defaults to number of logical CPUs - 2"
  )]
  pub workers: Option<usize>,
}

pub struct App {
  pub args: Config,
  interactive: bool,
}

impl App {
  pub fn new() -> AnyhowResult<Self> {
    let interactive =
      atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout);

    Ok(App {
      args: Config::parse(),
      interactive,
    })
  }

  fn unwrap_or_else_fn<T>(error: InquireError) -> T {
    match error {
      InquireError::OperationCanceled => {
        process::exit(0);
      }
      InquireError::OperationInterrupted => {
        println!("Interrupted! aborting...");

        process::exit(1);
      }
      _ => {
        panic!("{}", error);
      }
    }
  }

  fn check_interactive(&mut self) {
    if !self.interactive {
      panic!("Cannot prompt in non-interactive mode (hint: fill in the missing arguments)");
    }
  }

  fn prompt_tries(&mut self) -> usize {
    Self::check_interactive(self);

    let tries: Result<usize, inquire::error::InquireError> =
      CustomType::new("How many tries:")
        .with_error_message("Please type a valid number")
        .prompt();

    tries.unwrap_or_else(Self::unwrap_or_else_fn)
  }

  fn prompt_times(&mut self) -> usize {
    Self::check_interactive(self);

    let times: Result<usize, inquire::error::InquireError> =
      CustomType::new("Wakuchins times:")
        .with_error_message("Please type a valid number")
        .prompt();

    times.unwrap_or_else(Self::unwrap_or_else_fn)
  }

  fn prompt_regex(&mut self) -> Regex {
    Self::check_interactive(self);

    let regex = Text::new("Regex to detect hits:")
      .with_validator(&|s| {
        if s.is_empty() {
          Err("Regex is empty".into())
        } else if Regex::new(s).is_err() {
          Err("Regex is invalid".into())
        } else {
          Ok(())
        }
      })
      .prompt();

    Regex::new(&regex.unwrap_or_else(Self::unwrap_or_else_fn))
      .expect("regular expression check has bypassed")
  }

  pub async fn prompt(&mut self) -> Config {
    let args_config_ref = self.args.config.as_ref();

    if args_config_ref.unwrap_or(&"".to_string()) != "" {
      let config = load_config(Path::new(
        &args_config_ref.expect("if check has bypassed"),
      ))
      .await
      .unwrap_or_else(|e| {
        panic!("error when parsing config: {}", e);
      });

      self.args.tries = self.args.tries.or(config.tries);
      self.args.times = self.args.times.or(config.times);
      self.args.regex =
        self.args.regex.as_ref().or(config.regex.as_ref()).cloned();
      self.args.out = self.args.out.as_ref().or(config.out.as_ref()).cloned();
    }

    if self.args.tries.is_none() {
      self.args.tries = Some(self.prompt_tries());
    }

    if self.args.times.is_none() {
      self.args.times = Some(self.prompt_times());
    }

    if self.args.regex.is_none() {
      self.args.regex = Some(self.prompt_regex());
    }

    self.args.out = self
      .args
      .out
      .as_ref()
      .or(Some(&ResultOutputFormat::Text))
      .cloned();

    self.args.clone()
  }
}
