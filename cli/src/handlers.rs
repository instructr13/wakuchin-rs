use std::io::stderr;
use std::time::Duration;

use clap::ValueEnum;
use crossterm::cursor::{Hide, MoveLeft, MoveUp, Show};
use crossterm::execute;
use crossterm::style::{Attribute, Print, Stylize};
use crossterm::terminal::{size as terminal_size, Clear, ClearType};

use serde::{Deserialize, Serialize};
use wakuchin::convert::chars_to_wakuchin;
use wakuchin::handlers::ProgressHandler;
use wakuchin::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use wakuchin::result::HitCount;

type Result<T> = anyhow::Result<T>;

const PROGRESS_BAR_WIDTH: u16 = 33;
const BOLD_START: Attribute = Attribute::Bold;
const BOLD_END: Attribute = Attribute::Reset;

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
  MsgpackBase64,
}

impl Default for HandlerKind {
  fn default() -> Self {
    Self::Console
  }
}

pub(crate) struct ConsoleProgressHandler {
  no_progress: bool,
  handler_height: usize,
  tries: usize,
  tries_string: String,
  times: usize,
}

impl ConsoleProgressHandler {
  pub(crate) fn new(no_progress: bool, tries: usize, times: usize) -> Self {
    Self {
      no_progress,
      handler_height: 0,
      tries,
      tries_string: format!("{}", tries),
      times,
    }
  }

  fn render_hit_counters(
    &self,
    buf: &mut itoa::Buffer,
    counters: &[HitCount],
  ) -> usize {
    let mut current_hit_total = 0;

    let tries_width = self.tries_string.len();

    for counter in counters {
      let chars = chars_to_wakuchin(&counter.chars).dim();
      let count = counter.hits;

      current_hit_total += count;

      eprintln!(
        "        {} {chars}: {BOLD_START}{:<tries_width$}{BOLD_END} ({:.3}%)",
        "hits".blue().underlined(),
        buf.format(count),
        count as f64 / self.tries as f64 * 100.0,
      );
    }

    eprintln!(
      "  {} {BOLD_START}{:<tries_width$}{BOLD_END} / {tries} ({:.3}%)",
      "total hits".blue().underlined(),
      buf.format(current_hit_total),
      current_hit_total as f64 / self.tries as f64 * 100.0,
      tries = self.tries
    );

    current_hit_total
  }

  fn render_workers(
    &self,
    buf: &mut itoa::Buffer,
    progresses: &[Progress],
  ) -> usize {
    let mut current_total = 0;

    let tries_width = self.tries_string.len();
    let mut id_width: usize = 0;

    for progress in progresses {
      match progress {
        Progress(ProgressKind::Idle(IdleDetail {
          id: 0,
          total_workers: 1,
        })) => {
          eprintln!("{}", "Idle".yellow());
        }
        Progress(ProgressKind::Idle(IdleDetail { id, total_workers })) => {
          if id_width == 0 {
            id_width = total_workers.to_string().len();
          }

          eprintln!(
            "{BOLD_START}#{id:<id_width$}{BOLD_END} {}",
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
                buf.format(*current)
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
                "{BOLD_START}#{id:<id_width$}{BOLD_END} {} {} • {:<tries_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).dim(),
                buf.format(*current)
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
            " ".repeat(self.times * 8 + self.tries_string.len() * 2 + 5),
          );
        }
        Progress(ProgressKind::Done(DoneDetail {
          id,
          total,
          total_workers,
        })) => {
          current_total += total;

          if id_width == 0 {
            id_width = total_workers.to_string().len();
          }

          eprintln!(
            "{BOLD_START}#{id:<id_width$}{BOLD_END} {} {}",
            "Done      ".green(),
            " ".repeat(self.times * 8 + self.tries_string.len() * 2 + 5),
          );
        }
      }
    }

    current_total
  }

  fn render_progress_segment(&self, width: usize, percentage: f64) -> String {
    if percentage >= 100.0 {
      "━".repeat(width).blue().to_string()
    } else {
      let block = (width as f64 * percentage / 100.0) as usize;
      let current = "━".repeat(block) + "╸";
      let space = width - block - 1;

      format!(
        "{}{}",
        if space == 0 {
          current.green()
        } else {
          current.blue()
        },
        "━".repeat(space).dim()
      )
    }
  }

  /// Use blue bar to indicate progress that is processing.
  /// Use green bar to indicate progress that is done.
  fn render_progress_bar(
    &self,
    buf: &mut itoa::Buffer,
    current: usize,
    elapsed_time: Duration,
    current_diff: usize,
  ) -> Result<()> {
    let tries_width = self.tries_string.len();
    let possible_bar_width = terminal_size()?.0 - tries_width as u16 * 2 - 55;

    let bar_width = if PROGRESS_BAR_WIDTH > possible_bar_width {
      possible_bar_width
    } else {
      PROGRESS_BAR_WIDTH
    };

    let percentage = current as f64 / self.tries as f64 * 100.0;
    let bar = self.render_progress_segment(bar_width.into(), percentage);
    let rate = current_diff as f32 / elapsed_time.as_secs_f32();
    let eta = (self.tries - current) as f32 / rate;

    execute!(
      stderr(),
      Print("Status ".bold()),
      Print(format!(
        "{bar} • {}: {BOLD_START}{:<tries_width$}{BOLD_END} / {tries} ({percentage:.0}%, {rate}/sec, eta: {eta:>3.0}sec)   ",
        "total".green().underlined(),
        buf.format(current),
        tries = self.tries,
        rate = human_format::Formatter::new().format(rate.into()),
      ))
    )?;

    Ok(())
  }
}

impl ProgressHandler for ConsoleProgressHandler {
  fn before_start(&self) -> anyhow::Result<()> {
    if !self.no_progress {
      execute!(
        stderr(),
        Hide,
        Print("Spawning workers..."),
        MoveLeft(u16::MAX)
      )?;
    }

    Ok(())
  }

  fn handle(
    &mut self,
    progresses: &[Progress],
    counters: &[HitCount],
    elapsed_time: Duration,
    current_diff: usize,
    all_done: bool,
  ) -> anyhow::Result<()> {
    if self.no_progress {
      return Ok(());
    }

    if self.handler_height == 0 {
      self.handler_height = progresses.len() + counters.len() + 1;
    } else {
      execute!(
        stderr(),
        MoveLeft(u16::MAX),
        MoveUp(self.handler_height as u16)
      )?;
    }

    let mut itoa_buf = itoa::Buffer::new();

    self.render_hit_counters(&mut itoa_buf, counters);

    let current_total = self.render_workers(&mut itoa_buf, progresses);

    if all_done {
      execute!(
        stderr(),
        Clear(ClearType::CurrentLine),
        Print("Status ".bold()),
        Print("All Done".bold().green())
      )?;

      return Ok(());
    }

    self.render_progress_bar(
      &mut itoa_buf,
      current_total,
      elapsed_time,
      current_diff,
    )?;

    Ok(())
  }

  fn after_finish(&self) -> anyhow::Result<()> {
    if !self.no_progress {
      for _ in 0..self.handler_height {
        execute!(
          stderr(),
          Clear(ClearType::CurrentLine),
          MoveUp(1),
          Clear(ClearType::CurrentLine)
        )?;
      }
    }

    execute!(stderr(), MoveLeft(u16::MAX), Show)?;

    Ok(())
  }
}
