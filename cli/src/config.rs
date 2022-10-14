use std::borrow::Borrow;
use std::path::Path;

use format_serde_error::SerdeError;
use tokio::fs::read_to_string;

use crate::app::Config;
use crate::error::{AppError, Result};

pub(crate) async fn load_config(path: &Path) -> Result<Config> {
  let contents =
    read_to_string(path)
      .await
      .map_err(|e| AppError::ConfigIoError {
        path: path.into(),
        source: e,
      })?;

  let config: Config = match path
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

  Ok(config)
}

#[cfg(test)]
mod test {
  use std::path::{Path, PathBuf};
  use std::time::Duration;

  use anyhow::Result;
  use wakuchin::result::ResultOutputFormat;

  use crate::error::AppError;
  use crate::handlers::HandlerKind;

  fn init() {
    format_serde_error::never_color();
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
      assert!(source.to_string().contains("invalid regex"));
    } else {
      panic!("Unexpected error: {:?}", invalid_regex_yaml_err);
    }

    let mut correct_toml = PathBuf::from(base_path);

    correct_toml.push("../examples/tries-300000000.toml");

    let config = super::load_config(&correct_toml).await?;

    assert_eq!(config.tries, Some(300000000));
    assert_eq!(config.times, Some(2));
    assert!(config.regex.is_some());

    if let Some(regex) = config.regex {
      assert_eq!(regex.as_str(), "(WKNCWKNC|WCKNWCKN)");
    }

    assert_eq!(config.out, ResultOutputFormat::Text);
    assert_eq!(config.config, None);
    assert_eq!(config.interval, Duration::from_millis(300));
    assert_eq!(config.workers, 0);
    assert_eq!(config.handler, HandlerKind::Console);

    Ok(())
  }
}
