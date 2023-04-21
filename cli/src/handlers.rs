use std::time::Duration;

use clap::ValueEnum;
use console::Term;
use owo_colors::OwoColorize as _;
use serde::{Deserialize, Serialize};
use wakuchin::convert::chars_to_wakuchin;
use wakuchin::handlers::ProgressHandler;
use wakuchin::progress::{
  DoneDetail, IdleDetail, ProcessingDetail, Progress, ProgressKind,
};
use wakuchin::result::HitCount;

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
pub enum HandlerKind {
  Console,
  Msgpack,
  MsgpackBase64,
}

impl Default for HandlerKind {
  fn default() -> Self {
    Self::Console
  }
}

pub struct ConsoleProgressHandler {
  no_progress: bool,
  handler_height: usize,
  term: Term,
  tries: usize,
  tries_string: String,
  times: usize,
  total_workers: usize,
}

impl ConsoleProgressHandler {
  pub fn new(no_progress: bool, tries: usize, times: usize) -> Self {
    Self {
      no_progress,
      handler_height: 0,
      term: Term::stderr(),
      tries,
      tries_string: tries.to_string(),
      times,
      total_workers: 0,
    }
  }

  fn append_id(
    id: usize,
    id_width: usize,
    base: impl Into<String>,
  ) -> (bool, String) {
    let base = base.into();

    if id == 0 {
      return (true, base);
    }

    (
      false,
      format!(
        "{} {base}",
        format!("#{id:<id_width$}").bold()
      ),
    )
  }

  fn pad_id(id: usize, id_width: usize, base: impl Into<String>) -> String {
    let base = base.into();

    if id == 0 {
      return base;
    }

    let actual_width = id_width + 2; // # + space

    format!("{}{base}", " ".repeat(actual_width))
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
    id_width: usize,
    hit_counts: &[HitCount],
  ) -> usize {
    let mut current_hit_total = 0;

    let tries_width = self.tries_string.len();

    for hit_count in hit_counts {
      let chars = chars_to_wakuchin(&hit_count.chars);
      let count = hit_count.hits;

      current_hit_total += count;

      eprintln!(
        "      {} {}: {:<} ({:.3}%)",
        Self::pad_id(
          self.total_workers,
          id_width,
          "hits".blue().underline().to_string(),
        ),
        chars.dimmed(),
        buf.format(count).bold(),
        count as f64 / self.tries as f64 * 100.0,
      );
    }

    eprintln!(
      "{} {:<tries_width$} / {tries} ({:.3}%)",
      Self::pad_id(
        self.total_workers,
        id_width,
        "total hits".blue().underline().to_string()
      ),
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
    let id_width = self.total_workers.to_string().len();

    for progress in progresses {
      let (sequential, body) = match progress {
        Progress(ProgressKind::Idle(IdleDetail { id })) => {
          Self::append_id(*id, id_width, "Idle".yellow().to_string())
        }
        Progress(ProgressKind::Processing(ProcessingDetail {
          id,
          current,
          total,
          wakuchin,
        })) => {
          current_total += current;

          Self::append_id(
            *id,
            id_width,
            format!(
              "{} {} • {:<tries_width$} / {total}",
              "Processing".blue(),
              chars_to_wakuchin(wakuchin).dimmed(),
              buf.format(*current)
            ),
          )
        }
        Progress(ProgressKind::Done(DoneDetail { id, total })) => {
          current_total += total;

          Self::append_id(
            *id,
            id_width,
            format!(
              "{} {}",
              "Done      ".green(),
              " ".repeat(self.times * 8 + self.tries_string.len() * 2 + 5)
            ),
          )
        }
      };

      if sequential {
        eprintln!(
          "{}",
          Self::pad_id(1, self.total_workers.to_string().len(), body)
        );
      } else {
        eprintln!("{body}");
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
  ) {
    let tries_width = self.tries_string.len();

    let possible_bar_width = {
      let size = self.term.size_checked();

      if let Some((width, _)) = size {
        if width < tries_width as u16 || width + (tries_width as u16) < 45 {
          PROGRESS_BAR_WIDTH
        } else {
          width - tries_width as u16 * 2 - 55
        }
      } else {
        PROGRESS_BAR_WIDTH
      }
    };

    let bar_width = if PROGRESS_BAR_WIDTH > possible_bar_width {
      possible_bar_width
    } else {
      PROGRESS_BAR_WIDTH
    };

    let id_width = self.total_workers.to_string().len();
    let percentage = current as f64 / self.tries as f64 * 100.0;
    let bar = Self::render_progress_segment(bar_width.into(), percentage);
    let rate = current_diff as f64 / elapsed_time.as_secs_f64();
    let eta = (self.tries - current) as f64 / rate;

    eprintln!(
      "{} {bar} • {}: {:<tries_width$} / {tries} ({percentage:.0}%, {rate}/sec, eta: {eta:>3.0}sec)   ",
      Self::pad_id(self.total_workers, id_width, "Status".bold().to_string()),
      "total".green().underline(),
      buf.format(current).bold(),
      tries = self.tries,
      rate = human_format::Formatter::new().format(rate),
    );
  }
}

impl ProgressHandler for ConsoleProgressHandler {
  fn before_start(&mut self, total_workers: usize) -> anyhow::Result<()> {
    if self.no_progress {
      return Ok(());
    }

    eprint!("Spawning workers...");

    self.term.hide_cursor()?;
    self.term.move_cursor_left(u16::MAX as usize)?;

    self.total_workers = total_workers;

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
      self.handler_height = self.total_workers + hit_counts.len() + 2;
    } else {
      self.term.move_cursor_left(u16::MAX as usize)?;
      self
        .term
        .move_cursor_up(self.handler_height as u16 as usize)?;
    }

    let mut itoa_buf = itoa::Buffer::new();

    self.render_hit_counts(
      &mut itoa_buf,
      self.total_workers.to_string().len(),
      hit_counts,
    );

    let current_total = self.render_workers(&mut itoa_buf, progresses);

    if all_done {
      self.term.clear_line()?;
      eprintln!("{} {}", "Status".bold(), "All Done".bold().green());

      return Ok(());
    }

    self.render_progress_bar(
      &mut itoa_buf,
      current_total,
      elapsed_time,
      current_diff,
    );

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
