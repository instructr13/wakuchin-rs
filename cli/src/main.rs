mod app;
mod config;
mod error;
mod handlers;

use std::io::stdout;
use std::panic;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use owo_colors::OwoColorize as _;
use wakuchin::builder::ResearchBuilder;
use wakuchin::error::WakuchinError;
use wakuchin::handlers::msgpack::{
  MsgpackBase64ProgressHandler, MsgpackProgressHandler,
};

use crate::app::App;
use crate::handlers::{ConsoleProgressHandler, HandlerKind};

#[cfg(all(
  not(target_os = "android"),
  not(target_env = "msvc"),
  not(target_arch = "wasm32")
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  if let Err(err) = try_main().await {
    if let Some(WakuchinError::Cancelled) = err.downcast_ref::<WakuchinError>()
    {
      std::process::exit(1);
    }

    eprintln!("{} {err}", "error:".red().bold());

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
        builder.progress_handler(Box::new(ConsoleProgressHandler::new(
          config.no_progress,
          config.tries,
          config.times,
        )))
      }
      HandlerKind::Msgpack => {
        builder.progress_handler(Box::new(MsgpackProgressHandler::new(
          config.tries,
          Arc::new(Mutex::new(stdout())),
        )))
      }
      HandlerKind::MsgpackBase64 => {
        builder.progress_handler(Box::new(MsgpackBase64ProgressHandler::new(
          config.tries,
          Arc::new(Mutex::new(stdout())),
        )))
      }
    }
  };

  #[cfg(not(any(feature = "sequential", target_arch = "wasm32")))]
  let result = builder.workers(config.workers).run_par();

  #[cfg(any(feature = "sequential", target_arch = "wasm32"))]
  let result = builder.run_seq();

  let result = result?;

  panic::set_hook(default_hook);

  println!("{}", result.out(config.out.into())?);

  Ok(())
}
