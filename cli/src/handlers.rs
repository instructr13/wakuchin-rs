use std::time::Duration;

use clap::ValueEnum;
use console::Term;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use wakuchin::convert::chars_to_wakuchin;
use wakuchin::handlers::ProgressHandler;
use wakuchin::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use wakuchin::result::HitCount;

type Result<T> = anyhow::Result<T>;

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
  MsgpackBase64,
}

impl Default for HandlerKind {
  fn default() -> Self {
    Self::Console
  }
}

pub(crate) struct ConsoleProgressHandler {
  no_progress: bool,
  handler_height: u16,
  term: Term,
  tries: usize,
  tries_string: String,
  times: usize,
}

impl ConsoleProgressHandler {
  pub(crate) fn new(no_progress: bool, tries: usize, times: usize) -> Self {
    Self {
      no_progress,
      handler_height: 0,
      term: Term::stderr(),
      tries,
      tries_string: format!("{tries}"),
      times,
    }
  }

  fn render_progress_segment(width: usize, percentage: f64) -> String {
    if percentage >= 100.0 {
      "━".repeat(width).blue().to_string()
    } else {
      let block = (width as f64 * percentage / 100.0) as usize;
      let current = "━".repeat(block) + "╸";
      let space = width - block - 1;

      format!(
        "{}{}",
        if space == 0 {
          current.green().to_string()
        } else {
          current.blue().to_string()
        },
        "━".repeat(space).dimmed()
      )
    }
  }

  fn render_hit_counts(
    &self,
    buf: &mut itoa::Buffer,
    hit_counts: &[HitCount],
  ) -> usize {
    let mut current_hit_total = 0;

    let tries_width = self.tries_string.len();

    for hit_count in hit_counts {
      let chars = chars_to_wakuchin(&hit_count.chars);
      let count = hit_count.hits;

      current_hit_total += count;

      eprintln!(
        "        {} {}: {:<} ({:.3}%)",
        "hits".underline().blue(),
        chars.dimmed(),
        buf.format(count).bold(),
        count as f64 / self.tries as f64 * 100.0,
      );
    }

    eprintln!(
      "  {} {:<tries_width$} / {tries} ({:.3}%)",
      "total hits".blue().underline(),
      buf.format(current_hit_total).bold(),
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
            "{} {}",
            format!("#{id:<id_width$}").bold(),
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
                chars_to_wakuchin(&processing_detail.wakuchin).dimmed(),
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
                "{} {} {} • {:<tries_width$} / {total}",
                format!("#{id:<id_width$}").bold(),
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).dimmed(),
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
            "{} {} {}",
            format!("#{id:<id_width$}").bold(),
            "Done      ".green(),
            " ".repeat(self.times * 8 + self.tries_string.len() * 2 + 5),
          );
        }
      }
    }

    current_total
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
    let possible_bar_width = {
      let size = self.term.size_checked();

      if let Some((width, _)) = size {
        width - tries_width as u16 * 2 - 55
      } else {
        PROGRESS_BAR_WIDTH
      }
    };

    let bar_width = if PROGRESS_BAR_WIDTH > possible_bar_width {
      possible_bar_width
    } else {
      PROGRESS_BAR_WIDTH
    };

    let percentage = current as f64 / self.tries as f64 * 100.0;
    let bar = Self::render_progress_segment(bar_width.into(), percentage);
    let rate = current_diff as f64 / elapsed_time.as_secs_f64();
    let eta = (self.tries - current) as f64 / rate;

    eprint!(
        "{} {bar} • {}: {:<tries_width$} / {tries} ({percentage:.0}%, {rate}/sec, eta: {eta:>3.0}sec)   ",
        "Status".bold(),
        "total".green().underline(),
        buf.format(current).bold(),
        tries = self.tries,
        rate = human_format::Formatter::new().format(rate),
      );

    Ok(())
  }
}

impl ProgressHandler for ConsoleProgressHandler {
  fn before_start(&mut self) -> anyhow::Result<()> {
    if !self.no_progress {
      eprint!("Spawning workers...");

      self.term.hide_cursor()?;
      self.term.move_cursor_left(u16::MAX as usize)?;
    }

    Ok(())
  }

  fn handle(
    &mut self,
    progresses: &[Progress],
    hit_counts: &[HitCount],
    elapsed_time: Duration,
    current_diff: usize,
    all_done: bool,
  ) -> anyhow::Result<()> {
    if self.no_progress {
      return Ok(());
    }

    if self.handler_height == 0 {
      let progresses_len: u16 = progresses
        .len()
        .try_into()
        .expect("Too many progresses to display");
      let hit_counts_len: u16 = hit_counts
        .len()
        .try_into()
        .expect("Too many hit counts to display");

      self.handler_height = progresses_len + hit_counts_len + 1;
    } else {
      self.term.move_cursor_left(usize::MAX)?;
      self.term.move_cursor_up(self.handler_height as usize)?;
    }

    let mut itoa_buf = itoa::Buffer::new();

    self.render_hit_counts(&mut itoa_buf, hit_counts);

    let current_total = self.render_workers(&mut itoa_buf, progresses);

    if all_done {
      self.term.clear_line()?;
      eprint!("{} {}", "Status".bold(), "All Done".bold().green());

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

  fn after_finish(&mut self) -> anyhow::Result<()> {
    if !self.no_progress {
      for _ in 0..self.handler_height {
        self.term.clear_last_lines(1)?;
        self.term.clear_line()?;
      }
    }

    self.term.move_cursor_left(u16::MAX as usize)?;
    self.term.show_cursor()?;

    Ok(())
  }
}
