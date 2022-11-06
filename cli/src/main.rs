mod app;
mod config;
mod error;
mod handlers;

use std::io::{stderr, stdout};
use std::panic;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::style::{Print, Stylize};
use crossterm::{cursor, execute};

use wakuchin::builder::ResearchBuilder;
use wakuchin::error::WakuchinError;
use wakuchin::handlers::msgpack::{
  MsgpackBase64ProgressHandler, MsgpackProgressHandler,
};

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
    if let Some(WakuchinError::Cancelled) = err.downcast_ref::<WakuchinError>()
    {
      std::process::exit(1);
    }

    eprintln!("{} {}", "error:".red().bold(), err);

    std::process::exit(1);
  }
}

async fn try_main() -> Result<()> {
  let mut app = App::new();

  app.setup_config().await?;

  let config = app.config;

  let default_hook = App::set_panic_hook();

  let builder = ResearchBuilder::new()
    .tries(config.tries)
    .times(config.times)
    .regex(config.regex)
    .progress_interval(config.interval);

  let builder = {
    match config.handler {
      HandlerKind::Console => {
        builder.progress_handler(ConsoleProgressHandler::new(
          config.no_progress,
          config.tries,
          config.times,
        ))
      }
      HandlerKind::Msgpack => {
        builder.progress_handler(MsgpackProgressHandler::new(
          config.tries,
          Arc::new(Mutex::new(stdout())),
        ))
      }
      HandlerKind::MsgpackBase64 => {
        builder.progress_handler(MsgpackBase64ProgressHandler::new(
          config.tries,
          Arc::new(Mutex::new(stdout())),
        ))
      }
    }
  };

  #[cfg(not(feature = "sequential"))]
  let result = builder.workers(config.workers).run_par().await;

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

  println!("{}", result.out(config.out.into())?);

  Ok(())
}
