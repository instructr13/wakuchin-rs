use std::panic::{self, PanicInfo};
use std::path::PathBuf;
use std::process;

use anyhow::anyhow;
use clap::Parser;
use clap_serde_derive::ClapSerde;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Input};
use regex::Regex;

use crate::config::{load_config, Config};
use crate::error::Result;

#[cfg(not(target_arch = "wasm32"))]
use shadow_rs::shadow;

#[cfg(not(target_arch = "wasm32"))]
shadow!(build);

const LONG_VERSION: Option<&'static str> = if cfg!(not(target_arch = "wasm32"))
{
  Some(build::CLAP_LONG_VERSION)
} else {
  None
};

#[derive(Parser)]
#[command(author, version, about, long_about = "A next generation wakuchin researcher software written in Rust
P2P-Develop

Wakuchin will generate shuffled \"わくちん\" characters and check whether they match a given regex.
If they do, it will print the result to stdout.",
long_version = LONG_VERSION,
after_long_help = "For more information, see GitHub repository: https://github.com/P2P-Develop/wakuchin-rs")]
struct Args {
  /// Config file path, can be json, yaml, or toml, detected by extension
  #[arg(value_name = "FILE")]
  config_path: Option<PathBuf>,

  /// Rest of arguments
  #[command(flatten)]
  config: <Config as ClapSerde>::Opt,
}

pub struct App {
  pub config: Config,
  args: Args,
  interactive: bool,
}

impl App {
  pub fn new() -> Self {
    let interactive =
      atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stderr);

    Self {
      config: Config::default(),
      args: Args::parse(),
      interactive,
    }
  }

  fn check_interactive(&self) {
    if !self.interactive {
      if cfg!(target_arch = "wasm32") {
        eprintln!("error: Cannot prompt in WebAssembly runtime (hint: pass arguments or config path to run)");
      } else {
        eprintln!("error: Cannot prompt in non-interactive mode (hint: pipe stdin/stderr to tty or fill in the missing arguments)");
      }

      process::exit(1);
    }
  }

  fn prompt_tries(&self, term: &Term) -> Result<usize> {
    self.check_interactive();

    Input::<usize>::with_theme(&ColorfulTheme::default())
      .with_prompt("How many tries:")
      .interact_on(term)
      .map_err(Into::into)
  }

  fn prompt_times(&self, term: &Term) -> Result<usize> {
    self.check_interactive();

    Input::<usize>::with_theme(&ColorfulTheme::default())
      .with_prompt("Wakuchins times:")
      .interact_on(term)
      .map_err(Into::into)
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

  pub fn set_panic_hook() -> Box<dyn Fn(&PanicInfo) + Send + Sync + 'static> {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(|panic_info| {
      let term = Term::buffered_stderr();

      term.show_cursor().unwrap();
      term.move_cursor_up(1).unwrap();
      term.move_cursor_left(u16::MAX as usize).unwrap();

      term.flush().unwrap();

      eprintln!(
        "wakuchin has panicked.\nPlease report this to the author:\n{}",
        panic_info,
      );

      process::exit(1);
    }));

    default_hook
  }

  pub fn setup_config(&mut self) -> Result<()> {
    let mut config = if let Some(config_path) = &self.args.config_path {
      load_config(config_path.as_path())?.merge(&mut self.args.config)
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
