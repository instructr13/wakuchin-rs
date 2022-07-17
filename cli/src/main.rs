mod app;

use std::process;
use wakuchin_core::{result, worker};

use crate::app::App;

type Result<T> = anyhow::Result<T, Box<dyn std::error::Error>>;

pub async fn run() -> Result<bool> {
  let mut app = App::new()?;
  let args = app.prompt();

  let result = worker::run_par(
    args.tries.unwrap(),
    args.times.unwrap(),
    args.regex.unwrap(),
    |hit| {
      println!("{} hit on {}", hit.chars, hit.hit_on);
    },
  )
  .await;

  println!("{}", result::out(app.args.out, &result));

  Ok(true)
}

#[tokio::main]
pub async fn main() {
  let result = run().await;

  match result {
    Err(error) => {
      eprintln!("error: {}", error);

      process::exit(1);
    }
    Ok(false) => {
      process::exit(1);
    }
    Ok(true) => {
      process::exit(0);
    }
  }
}
