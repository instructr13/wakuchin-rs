use std::borrow::Borrow;
use std::path::Path;

use anyhow::{anyhow, Result};
use format_serde_error::SerdeError;
use tokio::fs::read_to_string;

use crate::app::Config;

pub(crate) async fn load_config(path: &Path) -> Result<Config> {
  let contents = read_to_string(path)
    .await
    .map_err(|e| anyhow!("'{}': {}", path.to_string_lossy(), e))?;

  let config: Config = match path
    .extension()
    .ok_or_else(|| {
      anyhow!("'{}': Invalid config type", path.to_string_lossy())
    })?
    .to_string_lossy()
    .borrow()
  {
    "json" => serde_json::from_str(&contents)
      .map_err(|e| SerdeError::new(contents, e))?,
    "yaml" | "yml" => serde_yaml::from_str(&contents)
      .map_err(|e| SerdeError::new(contents, e))?,
    "toml" => {
      toml::from_str(&contents).map_err(|e| SerdeError::new(contents, e))?
    }
    _ => Err(anyhow!("'{}': Invalid config type", path.to_string_lossy()))?,
  };

  Ok(config)
}

#[cfg(test)]
mod test {
  use std::path::{Path, PathBuf};
  use std::time::Duration;

  use anyhow::Result;
  use format_serde_error::SerdeError;
  use wakuchin::result::ResultOutputFormat;

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

    assert!(format!(
      "{}",
      invalid_regex_yaml_err.downcast_ref::<SerdeError>().unwrap()
    )
    .contains("regex parse error"));

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
