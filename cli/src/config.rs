use std::borrow::Borrow;
use std::path::Path;

use anyhow::{anyhow, Result};
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
    "json" => serde_json::from_str(&contents)?,
    "yaml" => serde_yaml::from_str(&contents)?,
    "toml" => toml::from_str(&contents)?,
    _ => Err(anyhow!("'{}': Invalid config type", path.to_string_lossy()))?,
  };

  Ok(config)
}
