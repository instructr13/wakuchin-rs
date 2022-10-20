use std::path::Path;
use std::{borrow::Borrow, time::Duration};

use clap::ValueEnum;
use clap_serde_derive::ClapSerde;
use format_serde_error::SerdeError;
use humantime::DurationError;
use regex::Regex;
use serde::Deserialize;
use tokio::fs::read_to_string;
use wakuchin::result::ResultOutputFormat;

use crate::error::{AppError, Result};
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

#[derive(Clone, Debug, ClapSerde)]
pub(crate) struct Config {
  /// Number of tries
  #[arg(short = 'i', long, value_name = "N")]
  pub(crate) tries: usize,

  /// Wakuchin times n
  ///
  /// Repeats "わくちん" n times.
  #[arg(short, long, value_name = "N")]
  pub(crate) times: usize,

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
  pub(crate) regex: Regex,

  // Result output format
  #[serde(rename(deserialize = "output"))]
  #[arg(short = 'f', long = "format", value_name = "FORMAT", value_enum)]
  pub(crate) out: InternalResultOutputFormat,

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
  pub(crate) interval: Duration,

  /// Progress handler to use
  ///
  /// Available handlers:
  ///  - "console": Prints progress to stderr with pretty progress bar
  ///  - "msgpack": Prints progress to stdout as raw msgpack-encoded data
  #[arg(short = 'H', long, value_enum, verbatim_doc_comment)]
  pub(crate) handler: HandlerKind,

  /// Do not show progress, able to use with --handler=console
  #[arg(long, value_name = "BOOL")]
  pub(crate) no_progress: bool,

  #[cfg(not(feature = "sequential"))]
  #[arg(
    short,
    long,
    value_name = "N",
    help = "Number of workers, 0 means number of logical CPUs"
  )]
  pub(crate) workers: usize,
}

pub(crate) async fn load_config(path: &Path) -> Result<Config> {
  let contents =
    read_to_string(path)
      .await
      .map_err(|e| AppError::ConfigIoError {
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
      AppError::ConfigDeserializeError {
        path: path.into(),
        line: Some(e.line()),
        column: Some(e.column()),
        source: SerdeError::new(contents, e),
      }
    })?,
    "yaml" | "yml" => serde_yaml::from_str(&contents).map_err(|e| {
      let location = e.location();

      let line = location.as_ref().map(|l| l.line()).as_ref().copied();
      let column = location.as_ref().map(|l| l.column()).as_ref().copied();

      AppError::ConfigDeserializeError {
        path: path.into(),
        line,
        column,
        source: SerdeError::new(contents, e),
      }
    })?,
    "toml" => toml::from_str(&contents).map_err(|e| {
      let (line, column) = e
        .line_col()
        .map(|(l, c)| (Some(l), Some(c)))
        .unwrap_or((None, None));

      AppError::ConfigDeserializeError {
        path: path.into(),
        line,
        column,
        source: SerdeError::new(contents, e),
      }
    })?,
    _ => Err(AppError::ConfigTypeNotSupported { path: path.into() })?,
  };

  Ok(Config::from(config))
}

#[cfg(test)]
mod test {
  use std::path::{Path, PathBuf};
  use std::time::Duration;

  use anyhow::Result;
  use humantime::DurationError;

  use crate::config::InternalResultOutputFormat;
  use crate::error::AppError;
  use crate::handlers::HandlerKind;

  fn init() {
    format_serde_error::never_color();
  }

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

  #[tokio::test]
  async fn test_load_config() -> Result<()> {
    init();

    let base_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut not_exist_json = PathBuf::from(base_path);

    not_exist_json.push("../examples/not_exist.json");

    let not_exist_json_err =
      super::load_config(&not_exist_json).await.unwrap_err();

    assert_eq!(
      not_exist_json_err.to_string(),
      format!(
        "'{}': No such file or directory (os error 2)",
        not_exist_json.to_string_lossy()
      )
    );

    let mut invalid_regex_yaml = PathBuf::from(base_path);

    invalid_regex_yaml.push("../examples/invalid-regex.yml");

    let invalid_regex_yaml_err =
      super::load_config(&invalid_regex_yaml).await.unwrap_err();

    if let AppError::ConfigDeserializeError { source, .. } =
      invalid_regex_yaml_err
    {
      assert!(source.to_string().contains("regex parse error"));
    } else {
      panic!("Unexpected error: {:?}", invalid_regex_yaml_err);
    }

    let mut correct_toml = PathBuf::from(base_path);

    correct_toml.push("../examples/tries-300000000.toml");

    let config = super::load_config(&correct_toml).await?;

    assert_eq!(config.tries, 300000000);
    assert_eq!(config.times, 2);
    assert_eq!(config.regex.as_str(), "(WKNCWKNC|WCKNWCKN)");

    assert_eq!(config.out, InternalResultOutputFormat::Text);
    assert_eq!(config.interval, Duration::from_millis(300));
    assert_eq!(config.workers, 0);
    assert_eq!(config.handler, HandlerKind::Console);

    Ok(())
  }
}
