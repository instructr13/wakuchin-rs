mod app;
mod config;
mod handlers;

use std::io::stderr;
use std::process;

use crossterm::{cursor, execute};

use wakuchin::progress::{HitCounter, Progress};
use wakuchin::result::out;
use wakuchin::worker;

use crate::app::App;
use crate::handlers::progress;

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

  execute!(stderr(), cursor::Hide)?;

  #[cfg(not(feature = "sequential"))]
  let result = worker::run_par(
    tries,
    args.times.expect("times is undefined"),
    args.regex.expect("regex is undefined"),
    progress::<&dyn Fn(&[Progress], &[HitCounter], bool)>(tries),
    None,
  )
  .await?;

  #[cfg(feature = "sequential")]
  let result = worker::run_seq(
    tries,
    args.times.expect("times is undefined"),
    args.regex.expect("regex is undefined"),
    progress::<&dyn Fn(&[Progress], &[HitCounter], bool)>(tries),
  )?;

  execute!(stderr(), cursor::Show)?;

  println!(
    "{}",
    out(app.args.out.expect("output format is undefined"), &result)
  );

  Ok(true)
}

#[tokio::main]
pub async fn main() {
  console_subscriber::init();

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
