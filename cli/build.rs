fn main() -> shadow_rs::SdResult<()> {
  #[cfg(not(target_arch = "wasm32"))]
  return shadow_rs::new();

  #[cfg(target_arch = "wasm32")]
  Ok(())
}
