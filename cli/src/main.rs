use wakuchin_core::{convert, gen};

#[tokio::main]
pub async fn main() {
  println!("Hello, world from wakuchin-rs/cli");
  println!("WKNCWKNC -> {}", convert::chars_to_wakuchin("WKNCWKNC"));
  println!("Random wakuchin: {}", convert::chars_to_wakuchin(&gen(2)))
}
