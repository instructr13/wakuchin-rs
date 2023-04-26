fn main() -> shadow_rs::SdResult<()> {
  if cfg!(not(target_arch = "wasm32")) {
    return shadow_rs::new();
  }

  Ok(())
}
