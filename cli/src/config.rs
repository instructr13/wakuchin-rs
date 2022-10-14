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
