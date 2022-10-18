use std::io::stderr;
use std::time::Duration;

use clap::ValueEnum;
use crossterm::cursor::{self, MoveLeft, MoveUp};
use crossterm::style::{Attribute, Print, Stylize};
use crossterm::terminal::ClearType;
use crossterm::{execute, terminal};

use serde::{Deserialize, Serialize};
use wakuchin::convert::chars_to_wakuchin;
use wakuchin::handlers::ProgressHandler;
use wakuchin::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use wakuchin::result::HitCounter;

const PROGRESS_BAR_WIDTH: u16 = 33;

#[derive(
  Clone,
  Debug,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Serialize,
  Deserialize,
  ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HandlerKind {
  Console,
  Msgpack,
}

impl Default for HandlerKind {
  fn default() -> Self {
    Self::Console
  }
}

pub(crate) struct ConsoleProgressHandler {
  no_progress: bool,
  tries: usize,
  times: usize,
}

impl ConsoleProgressHandler {
  pub(crate) fn new(no_progress: bool, tries: usize, times: usize) -> Self {
    Self {
      no_progress,
      tries,
      times,
    }
  }
}

impl ProgressHandler for ConsoleProgressHandler {
  fn before_start(&self) -> anyhow::Result<()> {
    if !self.no_progress {
      execute!(
        stderr(),
        cursor::Hide,
        Print("Spawning workers..."),
        cursor::MoveLeft(u16::MAX)
      )?;
    }

    Ok(())
  }

  fn handle(
    &mut self,
    progresses: &[Progress],
    counters: &[HitCounter],
    elapsed_time: Duration,
    current_diff: usize,
    all_done: bool,
  ) -> anyhow::Result<()> {
    let progress_len = progresses.len() + counters.len() + 1;
    let elapsed_time = elapsed_time.as_secs_f32();
    let tries_width = self.tries.to_string().len();
    let bold_start = Attribute::Bold;
    let bold_end = Attribute::Reset;

    let mut current_total = 0;
    let mut current_hit_total = 0;

    let mut itoa_buf = itoa::Buffer::new();

    for counter in counters {
      let chars = chars_to_wakuchin(&counter.chars).dim();
      let count = counter.hits;

      current_hit_total += count;

      eprintln!(
        "        {} {chars}: {bold_start}{:<tries_width$}{bold_end} ({:.3}%)",
        "hits".blue().underlined(),
        itoa_buf.format(count),
        count as f64 / self.tries as f64 * 100.0,
      );
    }

    eprintln!(
      "  {} {bold_start}{:<tries_width$}{bold_end} / {tries} ({:.3}%)",
      "total hits".blue().underlined(),
      itoa_buf.format(current_hit_total),
      current_hit_total as f64 / self.tries as f64 * 100.0,
      tries = self.tries
    );

    for progress in progresses {
      match progress {
        Progress(ProgressKind::Idle(IdleDetail {
          id: 0,
          total_workers: 1,
        })) => {
          eprintln!("{}", "Idle".yellow());
        }
        Progress(ProgressKind::Idle(IdleDetail { id, total_workers })) => {
          let id_width = total_workers.to_string().len();

          eprintln!(
            "{bold_start}#{id:<id_width$}{bold_end} {}",
            "Idle".yellow(),
          );
        }
        Progress(ProgressKind::Processing(processing_detail)) => {
          match processing_detail {
            ProcessingDetail {
              id: 0,
              current,
              total,
              total_workers: 1,
              ..
            } => {
              current_total += current;

              eprintln!(
                "{} {} • {:<tries_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).dim(),
                itoa_buf.format(*current)
              );
            }
            ProcessingDetail {
              id,
              current,
              total,
              total_workers,
              ..
            } => {
              current_total += current;

              let id_width = total_workers.to_string().len();

              eprintln!(
                "{bold_start}#{id:<id_width$}{bold_end} {} {} • {:<tries_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).dim(),
                itoa_buf.format(*current)
              );
            }
          }
        }
        Progress(ProgressKind::Done(DoneDetail {
          id: 0,
          total,
          total_workers: 1,
          ..
        })) => {
          current_total += total;

          eprintln!(
            "{} {}",
            "Done      ".green(),
            " ".repeat(self.times * 8 + self.tries.to_string().len() * 2 + 5),
          );
        }
        Progress(ProgressKind::Done(DoneDetail {
          id,
          total,
          total_workers,
        })) => {
          current_total += total;

          let id_width = total_workers.to_string().len();

          eprintln!(
            "{bold_start}#{id:<id_width$}{bold_end} {} {}",
            "Done      ".green(),
            " ".repeat(self.times * 8 + self.tries.to_string().len() * 2 + 5),
          );
        }
      }
    }

    if all_done {
      execute!(
        stderr(),
        terminal::Clear(ClearType::CurrentLine),
        Print("Status ".bold()),
        Print("All Done".bold().green())
      )?;

      for _ in 0..progress_len {
        execute!(
          stderr(),
          terminal::Clear(ClearType::CurrentLine),
          MoveUp(1),
          terminal::Clear(ClearType::CurrentLine)
        )?;
      }
    } else {
      let width = terminal::size()?.0 - tries_width as u16 * 2 - 55;

      let mut progress = String::new();

      let progress_size = if PROGRESS_BAR_WIDTH > width {
        width
      } else {
        PROGRESS_BAR_WIDTH
      };

      let progress_percentage =
        current_total as f64 / self.tries as f64 * 100.0;
      let progress_rate = current_diff as f32 / elapsed_time;
      let progress_remaining_time =
        (self.tries - current_total) as f32 / progress_rate;

      if progress_percentage >= 100.0 {
        progress.push_str(&"━".repeat(progress_size.into()).blue().to_string());
      } else {
        let block =
          (progress_size as f64 * progress_percentage / 100.0) as usize;

        progress.push_str(&("━".repeat(block) + "╸").blue().to_string());
        progress.push_str(
          &"━"
            .repeat(progress_size as usize - block - 1)
            .dim()
            .to_string(),
        );
      }

      execute!(
        stderr(),
        Print("Status ".bold()),
        Print(format!(
          "{progress} • {}: {bold_start}{:<tries_width$}{bold_end} / {tries} ({progress_percentage:.0}%, {progress_rate}/sec, eta: {progress_remaining_time:>3.0}sec)   ",
          "total".green().underlined(),
          itoa_buf.format(current_total),
          tries = self.tries,
          progress_rate = human_format::Formatter::new().format(progress_rate.into()),
        )),
        MoveLeft(u16::MAX),
        MoveUp(progress_len as u16)
      )?;
    }

    Ok(())
  }
}
