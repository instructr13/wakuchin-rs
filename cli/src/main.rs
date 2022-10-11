mod app;
mod config;
mod handlers;

use std::error::Error;
use std::io::stderr;
use std::panic;
use std::process;

use crossterm::style::Print;
use crossterm::style::Stylize;
use crossterm::{cursor, execute};

use wakuchin::builder::ResearchBuilder;

use crate::app::App;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

type Result<T> = anyhow::Result<T, Box<dyn Error>>;

async fn run() -> Result<bool> {
  let mut app = App::new()?;
  let args = app.prompt().await?;

  let default_hook = App::set_panic_hook();

  let tries = args.tries.ok_or("tries is required")?;
  let times = args.times.ok_or("times is required")?;

  let builder = ResearchBuilder::new()
    .tries(tries)
    .times(times)
    .regex(args.regex.ok_or("regex compilation failed")?)
    .progress_interval(args.interval)
    .progress_handler(handlers::progress(tries, times));

  execute!(
    stderr(),
    cursor::Hide,
    Print("Spawning workers..."),
    cursor::MoveLeft(u16::MAX)
  )?;

  #[cfg(not(feature = "sequential"))]
  let result = builder.workers(args.workers).run_par().await?;

  #[cfg(feature = "sequential")]
  let result = builder.run_seq()?;

  panic::set_hook(default_hook);

  println!(
    "{}",
    result.out(app.args.out.ok_or("output format is undefined")?)?
  );

  Ok(true)
}

#[tokio::main]
async fn main() {
  let result = run().await;

  execute!(stderr(), cursor::MoveLeft(u16::MAX), cursor::Show).unwrap_or_else(
    |_| {
      eprintln!("error: failed to restore cursor");

      process::exit(1)
    },
  );

  match result {
    Err(error) => {
      eprintln!("{} {error}", "error:".red());

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
