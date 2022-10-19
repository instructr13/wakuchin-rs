mod app;
mod config;
mod error;
mod handlers;

use std::io::{stderr, stdout};
use std::panic;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use crossterm::style::{Print, Stylize};
use crossterm::{cursor, execute};

use wakuchin::builder::ResearchBuilder;
use wakuchin::error::WakuchinError;
use wakuchin::handlers::msgpack::MsgpackProgressHandler;

use crate::app::App;
use crate::handlers::{ConsoleProgressHandler, HandlerKind};

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_os = "android"), not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
  if let Err(err) = try_main().await {
    eprintln!("{} {}", "error:".red().bold(), err);

    std::process::exit(1);
  }
}

async fn try_main() -> Result<()> {
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
    .progress_interval(args.interval);

  let builder = {
    match args.handler {
      HandlerKind::Console => builder.progress_handler(
        ConsoleProgressHandler::new(args.no_progress, tries, times),
      ),
      HandlerKind::Msgpack => builder.progress_handler(
        MsgpackProgressHandler::new(tries, Arc::new(Mutex::new(stdout()))),
      ),
    }
  };

  #[cfg(not(feature = "sequential"))]
  let result = builder.workers(args.workers).run_par().await;

  #[cfg(feature = "sequential")]
  let result = builder.run_seq();

  if result.is_err() {
    if let Err(WakuchinError::WorkerError(e)) = result {
      if !e.is_panic() {
        return Err(e.into());
      }

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

  println!("{}", result.out(args.out.into())?);

  execute!(stderr(), cursor::MoveLeft(u16::MAX), cursor::Show)?;

  Ok(())
}
