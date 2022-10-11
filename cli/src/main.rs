mod app;
mod config;
mod handlers;

use std::io::stderr;
use std::panic;

use anyhow::{anyhow, Result};
use crossterm::style::Print;
use crossterm::{cursor, execute};

use wakuchin::builder::ResearchBuilder;
use wakuchin::error::WakuchinError;

use crate::app::App;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> Result<()> {
  let mut app = App::new();
  let args = app.prompt().await?;

  let default_hook = App::set_panic_hook();

  let tries = args
    .tries
    .ok_or_else(|| anyhow!("tries is required but was undefined"))?;
  let times = args
    .times
    .ok_or_else(|| anyhow!("times is required but was undefined"))?;

  let builder = ResearchBuilder::new()
    .tries(tries)
    .times(times)
    .regex(
      args
        .regex
        .ok_or_else(|| anyhow!("regex compilation failed"))?,
    )
    .progress_interval(args.interval)
    .progress_handler(handlers::progress(tries, times));

  execute!(
    stderr(),
    cursor::Hide,
    Print("Spawning workers..."),
    cursor::MoveLeft(u16::MAX)
  )?;

  #[cfg(not(feature = "sequential"))]
  let result = builder.run_par().await;

  #[cfg(feature = "sequential")]
  let result = builder.run_seq();

  if result.is_err() {
    if let Err(WakuchinError::WorkerError(e)) = result {
      execute!(
        stderr(),
        cursor::Show,
        Print("\n"),
        cursor::MoveUp(1),
        cursor::MoveLeft(u16::MAX),
        Print("wakuchin has panicked.\n"),
        Print("Please report this to the author.\n"),
        Print(format!("{:?}", e)),
        cursor::MoveLeft(u16::MAX),
      )?;

      panic::resume_unwind(e.into_panic());
    }
  }

  let result = result?;

  panic::set_hook(default_hook);

  println!(
    "{}",
    result.out(app.args.out.ok_or_else(|| anyhow!(
      "output format is required but was undefined"
    ))?)?
  );

  execute!(stderr(), cursor::MoveLeft(u16::MAX), cursor::Show)?;

  Ok(())
}
