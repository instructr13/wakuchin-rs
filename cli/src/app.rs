use std::process;

use clap::Parser;
use inquire::{error::InquireError, CustomType, Text};
use regex::Regex;

use wakuchin_core::result::ResultOutputFormat;

type AnyhowResult<T> = anyhow::Result<T, Box<dyn std::error::Error>>;

fn value_parser_format(s: &str) -> Result<ResultOutputFormat, String> {
  match s {
    "text" => Ok(ResultOutputFormat::Text),
    "json" => Ok(ResultOutputFormat::Json),
    _ => Err(format!("Unknown format: {}", s).into()),
  }
}

#[derive(Clone, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
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

  #[clap(short, long, value_parser, help = "Regex to detect hits")]
  pub regex: Option<Regex>,

  #[clap(short = 'f', long = "format", default_value = "text", value_parser = value_parser_format, value_name = "text|json", help = "Output format")]
  pub out: ResultOutputFormat,
}

pub struct App {
  pub args: Args,
  interactive: bool,
}

impl App {
  pub fn new() -> AnyhowResult<Self> {
    let interactive =
      atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout);

    Ok(App {
      args: Args::parse(),
      interactive,
    })
  }

  fn check_interactive(&mut self) {
    if !self.interactive {
      panic!("Cannot prompt in non-interactive mode (hint: fill in the missing arguments)");
    }
  }

  fn check_prompt_err<T>(
    &mut self,
    result: &Result<T, inquire::error::InquireError>,
  ) {
    match result {
      Ok(_) => {}
      Err(InquireError::OperationCanceled) => {
        process::exit(0);
      }
      Err(InquireError::OperationInterrupted) => {
        println!("Interrupted! aborting...");

        process::exit(1);
      }
      Err(error) => {
        panic!("{}", error);
      }
    }
  }

  fn prompt_tries(&mut self) -> usize {
    Self::check_interactive(self);

    let tries: Result<usize, inquire::error::InquireError> =
      CustomType::new("How many tries:")
        .with_error_message("Please type a valid number")
        .prompt();

    self.check_prompt_err(&tries);

    tries.unwrap()
  }

  fn prompt_times(&mut self) -> usize {
    Self::check_interactive(self);

    let times: Result<usize, inquire::error::InquireError> =
      CustomType::new("Wakuchins times:")
        .with_error_message("Please type a valid number")
        .prompt();

    self.check_prompt_err(&times);

    times.unwrap()
  }

  fn prompt_regex(&mut self) -> Regex {
    Self::check_interactive(self);

    let regex = Text::new("Regex to detect hits:")
      .with_validator(&|s| {
        if s.is_empty() {
          Err("Regex is empty".into())
        } else if Regex::new(&s).is_err() {
          Err("Regex is invalid".into())
        } else {
          Ok(())
        }
      })
      .prompt();

    self.check_prompt_err(&regex);

    Regex::new(&regex.unwrap()).unwrap()
  }

  pub fn prompt(&mut self) -> Args {
    if self.args.tries.is_none() {
      self.args.tries = Some(self.prompt_tries());
    }

    if self.args.times.is_none() {
      self.args.times = Some(self.prompt_times());
    }

    if self.args.regex.is_none() {
      self.args.regex = Some(self.prompt_regex());
    }

    self.args.clone()
  }
}
