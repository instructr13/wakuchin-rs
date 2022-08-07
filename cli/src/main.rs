mod app;
mod config;
mod handlers;

use std::io::stderr;
use std::panic;
use std::process;
use std::time::Duration;

use crossterm::style::Print;
use crossterm::{cursor, execute};

use wakuchin::progress::Progress;
use wakuchin::result::{out, HitCounter};
use wakuchin::worker;

use crate::app::App;
use crate::handlers::progress;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

type Result<T> = anyhow::Result<T, Box<dyn std::error::Error>>;

pub async fn run() -> Result<bool> {
  let mut app = App::new()?;
  let args = app.prompt().await;
  let tries = args.tries.expect("tries is undefined");
  let times = args.times.expect("times is undefined");

  execute!(
    stderr(),
    cursor::Hide,
    Print("Spawning workers..."),
    cursor::MoveLeft(u16::MAX)
  )?;

  let default_hook = panic::take_hook();

  panic::set_hook(Box::new(|panic_info| {
    execute!(
      stderr(),
      cursor::Show,
      Print("\n"),
      cursor::MoveUp(1),
      cursor::MoveLeft(u16::MAX),
      Print("wakuchin has panicked.\n"),
      Print("Please report this to the author.\n"),
      Print(format!("{:?}", panic_info)),
      cursor::MoveLeft(u16::MAX),
    )
    .unwrap();

    process::exit(1);
  }));

  #[cfg(not(feature = "sequential"))]
  let result = worker::run_par(
    tries,
    times,
    args.regex.expect("regex is undefined"),
    progress::<&dyn Fn(&[Progress], &[HitCounter], Duration, usize, bool)>(
      tries, times,
    ),
    args.interval,
    None,
  )
  .await?;

  #[cfg(feature = "sequential")]
  let result = worker::run_seq(
    tries,
    args.times.expect("times is undefined"),
    args.regex.expect("regex is undefined"),
    progress::<&dyn Fn(&[Progress], &[HitCounter], Duration, usize, bool)>(
      tries, times,
    ),
    args.interval,
  )?;

  panic::set_hook(default_hook);

  execute!(stderr(), cursor::MoveLeft(u16::MAX), cursor::Show)?;

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
