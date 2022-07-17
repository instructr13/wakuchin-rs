use wakuchin_core::{convert, worker};

use regex::Regex;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
pub async fn main() -> Result<()> {
  println!("Hello, world from wakuchin-rs/cli");
  println!("WKNCWKNC -> {}", convert::chars_to_wakuchin("WKNCWKNC"));
  println!(
    "Result: {:#?}",
    worker::run_par(200000, 2, Regex::new(r"^WKNCWKNC$").unwrap()).await
  );

  Ok(())
}
