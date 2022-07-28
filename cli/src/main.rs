mod app;
mod config;
mod hit;

use std::process;

use wakuchin::result::{out, Hit};
use wakuchin::worker;

use crate::app::App;
use crate::hit::hit;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

type Result<T> = anyhow::Result<T, Box<dyn std::error::Error>>;

pub async fn run() -> Result<bool> {
  let mut app = App::new()?;
  let args = app.prompt().await;
  let tries = args.tries.expect("tries is undefined");

  let result = worker::run_par(
    tries,
    args.times.expect("times is undefined"),
    args.regex.expect("regex is undefined"),
    hit::<&dyn Fn(&Hit)>(tries),
  )?;

  println!(
    "{}",
    out(app.args.out.expect("output format is undefined"), &result)
  );

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
