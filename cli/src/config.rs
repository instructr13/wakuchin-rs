use std::borrow::Borrow;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Duration;

use clap::ValueEnum;
use clap_serde_derive::ClapSerde;
use humantime::DurationError;
use miette::{SourceOffset, SourceSpan};
use regex::Regex;
use serde::Deserialize;
use wakuchin::result::ResultOutputFormat;

use crate::error::{AppError, ConfigParseError, Result};
use crate::handlers::HandlerKind;

fn default_duration() -> Option<Duration> {
  Some(Duration::from_millis(300))
}

fn parse_duration(
  duration: &str,
) -> std::result::Result<Duration, DurationError> {
  Ok(duration.parse::<humantime::Duration>()?.into())
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize,
)]
pub enum InternalResultOutputFormat {
  #[serde(rename = "text")]
  #[default]
  Text,
  #[serde(rename = "json")]
  Json,
}

impl From<InternalResultOutputFormat> for ResultOutputFormat {
  fn from(format: InternalResultOutputFormat) -> Self {
    match format {
      InternalResultOutputFormat::Text => Self::Text,
      InternalResultOutputFormat::Json => Self::Json,
    }
  }
}

#[derive(Clone, Debug, ClapSerde)]
pub struct Config {
  /// Number of tries
  #[arg(short = 'i', long, value_name = "N")]
  pub tries: usize,

  /// Wakuchin times n
  ///
  /// Repeats "わくちん" n times.
  #[arg(short, long, value_name = "N")]
  pub times: usize,

  /// Regex to detect hits
  ///
  /// Following characters are recognized as:
  ///   W -> わ,
  ///   K -> く,
  ///   C -> ち,
  ///   N -> ん
  /// Used to check matches, characters of matches are ignored.
  #[default(Regex::new(r"%default%").unwrap())]
  #[serde(with = "serde_regex")]
  #[arg(short, long, verbatim_doc_comment)]
  pub regex: Regex,

  // Result output format
  #[serde(rename(deserialize = "output"))]
  #[arg(short = 'f', long = "format", value_name = "FORMAT", value_enum)]
  pub out: InternalResultOutputFormat,

  /// Progress refresh interval
  ///
  /// Can be passed as a human-readable duration, e.g. "1s", "2m", "3h", "4d".
  #[default(Duration::from_millis(300))]
  #[serde(with = "humantime_serde")]
  #[serde(default = "default_duration")]
  #[arg(
    short = 'd',
    long,
    value_name = "DURATION",
    value_parser = parse_duration
  )]
  pub interval: Duration,

  /// Progress handler to use
  ///
  /// Available handlers:
  ///  - "console": Prints progress to stderr with pretty progress bar
  ///  - "msgpack": Prints progress to stdout as raw msgpack-encoded data
  ///  - "msgpack-base64": Prints progress to stdout as base64-encoded msgpack data
  #[arg(short = 'H', long, value_enum, verbatim_doc_comment)]
  pub handler: HandlerKind,

  /// Do not show progress, able to use with --handler=console
  #[arg(long, value_name = "BOOL")]
  pub no_progress: bool,

  #[cfg(not(feature = "sequential"))]
  #[arg(
    short,
    long,
    value_name = "N",
    help = "Number of workers, 0 means number of logical CPUs"
  )]
  pub workers: usize,
}

/// `serde_json` and `serde_yaml` both append `" at line N column M"` to their
/// `Display`. miette renders the location itself, so drop the duplicate.
fn strip_location(message: &str, line: usize, column: usize) -> String {
  message
    .strip_suffix(&format!(" at line {line} column {column}"))
    .unwrap_or(message)
    .to_owned()
}

pub fn load_config(path: &Path) -> Result<Config> {
  let contents = read_to_string(path).map_err(|e| AppError::ConfigIoError {
    path: path.into(),
    source: e,
  })?;

  let config: <Config as ClapSerde>::Opt = match path
    .extension()
    .ok_or_else(|| AppError::ConfigTypeNotSupported { path: path.into() })?
    .to_string_lossy()
    .borrow()
  {
    "json" => serde_json::from_str(&contents).map_err(|e| {
      let (line, column) = (e.line(), e.column());

      // Both of these borrow `contents`, so they have to be bound before it is
      // moved into the error.
      let span: SourceSpan =
        SourceOffset::from_location(&contents, line, column).into();
      let reason = strip_location(&e.to_string(), line, column);

      ConfigParseError::new(path, contents, Some(span), reason)
    })?,
    "yaml" | "yml" => yaml_serde::from_str(&contents).map_err(|e| {
      let message = e.to_string();

      // `Location::index()` is documented as a byte index, but yaml-rust counts
      // chars, so derive the offset from line and column instead.
      let (span, reason): (Option<SourceSpan>, String) = match e.location() {
        Some(location) => {
          let (line, column) = (location.line(), location.column());

          (
            Some(SourceOffset::from_location(&contents, line, column).into()),
            strip_location(&message, line, column),
          )
        }
        None => (None, message),
      };

      ConfigParseError::new(path, contents, span, reason)
    })?,
    "toml" => toml::from_str(&contents).map_err(|e| {
      // toml reports a real byte range, and its `Display` is a whole
      // rustc-style block, so take the bare message.
      ConfigParseError::new(
        path,
        contents,
        e.span().map(SourceSpan::from),
        e.message(),
      )
    })?,
    _ => Err(AppError::ConfigTypeNotSupported { path: path.into() })?,
  };

  Ok(Config::from(config))
}

#[cfg(test)]
mod test {
  use std::time::Duration;

  use anyhow::Result;
  use humantime::DurationError;

  #[cfg(not(target_arch = "wasm32"))]
  use crate::config::InternalResultOutputFormat;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::error::AppError;
  #[cfg(not(target_arch = "wasm32"))]
  use crate::handlers::HandlerKind;
  #[cfg(not(target_arch = "wasm32"))]
  use std::path::{Path, PathBuf};

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

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn test_load_config() -> Result<()> {
    let base_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut not_exist_json = PathBuf::from(base_path);

    not_exist_json.push("../examples/not_exist.json");

    let not_exist_json_err = super::load_config(&not_exist_json).unwrap_err();

    assert_eq!(
      not_exist_json_err.to_string(),
      format!(
        "could not read config file '{}'",
        not_exist_json.to_string_lossy()
      )
    );

    let mut invalid_regex_yaml = PathBuf::from(base_path);

    invalid_regex_yaml.push("../examples/invalid-regex.yml");

    let invalid_regex_yaml_err =
      super::load_config(&invalid_regex_yaml).unwrap_err();

    match invalid_regex_yaml_err {
      AppError::ConfigParse(err) => {
        assert!(err.reason().contains("regex parse error"));
        // miette renders the location itself, so it must not be repeated.
        assert!(!err.reason().contains("at line"));
      }
      err => panic!("Unexpected error: {err:?}"),
    }

    let mut correct_toml = PathBuf::from(base_path);

    correct_toml.push("../examples/tries-300000000.toml");

    let config = super::load_config(&correct_toml)?;

    assert_eq!(config.tries, 300000000);
    assert_eq!(config.times, 2);
    assert_eq!(config.regex.as_str(), "(WKNCWKNC|WCKNWCKN)");

    assert_eq!(config.out, InternalResultOutputFormat::Text);
    assert_eq!(config.interval, Duration::from_millis(300));
    assert_eq!(config.workers, 0);
    assert_eq!(config.handler, HandlerKind::Console);

    Ok(())
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn test_load_config_syntax_error() {
    let mut syntax_error_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    syntax_error_json.push("../examples/syntax-error.json");

    let err = super::load_config(&syntax_error_json).unwrap_err();

    let AppError::ConfigParse(err) = err else {
      panic!("Unexpected error: {err:?}");
    };

    assert!(err.reason().contains("expected"));
    assert!(!err.reason().contains("at line"));

    let mut rendered = String::new();

    miette::GraphicalReportHandler::new_themed(
      miette::GraphicalTheme::unicode_nocolor(),
    )
    .with_width(80)
    .render_report(&mut rendered, &err)
    .unwrap();

    // serde_json reports the column of the character before the unexpected
    // one, so only the line is stable enough to assert on.
    assert!(rendered.contains("syntax-error.json:4:"));
    assert!(rendered.contains(r#""regex": "WKNCWKNC""#));
  }
}
