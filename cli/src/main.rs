mod app;
mod hit;

use std::process;

use wakuchin_core::result::{out, Hit};
use wakuchin_core::worker;

use crate::app::App;
use crate::hit::hit;

type Result<T> = anyhow::Result<T, Box<dyn std::error::Error>>;

pub async fn run() -> Result<bool> {
  let mut app = App::new()?;
  let args = app.prompt();
  let tries = args.tries.expect("tries is undefined");

  let result = worker::run_par(
    tries,
    args.times.expect("times is undefined"),
    args.regex.expect("regex is undefined"),
    hit::<&dyn Fn(&Hit)>(tries),
  )
  .await;

  println!("{}", out(app.args.out, &result));

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
