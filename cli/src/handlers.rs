use std::ops::{Bound, RangeBounds};
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

const DEFAULT_TERMINAL_WIDTH: u16 = 33;
const DEFAULT_TERMINAL_HEIGHT: u16 = 20;

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

  /// Append a worker ID to the base string.
  /// If the ID is 0, return true and the base string.
  ///
  /// # Arguments
  ///
  /// * `id` - Worker ID
  /// * `id_width` - Max width of the ID
  /// * `base` - Base string
  ///
  /// # Returns
  ///
  /// `(is_sequential, appended_string)`
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
      format!("{} {base}", format!("#{id:<id_width$}").bold()),
    )
  }

  fn append_id_range(
    id_range: impl RangeBounds<usize>,
    base: impl Into<String>,
  ) -> (bool, String) {
    let base = base.into();

    let (start, end) = match (id_range.start_bound(), id_range.end_bound()) {
      (Bound::Included(start), Bound::Included(end)) => (start, end),
      _ => unreachable!(),
    };

    match (start, end) {
      (0, 0) | (1, 1) => (true, base),
      _ => (
        false,
        format!("{} {base}", format!("#{}-{}", start, end).bold(),),
      ),
    }
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
    terminal_height: u16,
  ) -> usize {
    // truncate all progress with one line if the terminal height is too small
    if self.handler_height > terminal_height.into() {
      // collect total processing workers
      let (idle_workers, processing_workers, done_workers) = progresses
        .iter()
        .filter_map(|progress| {
          (
            matches!(progress, Progress(ProgressKind::Idle(_))),
            matches!(progress, Progress(ProgressKind::Processing(_))),
            matches!(progress, Progress(ProgressKind::Done(_))),
          )
            .into()
        })
        .fold(
          (0, 0, 0),
          |(idle_workers, processing_workers, done_workers),
           (is_idle, is_processing, is_done)| {
            (
              idle_workers + is_idle as usize,
              processing_workers + is_processing as usize,
              done_workers + is_done as usize,
            )
          },
        );

      fn truncate_if_zero(base: impl Into<String>, value: usize) -> String {
        if value == 0 {
          return "".to_string();
        }

        base.into()
      }

      fn make_workers_count_item(
        name: impl Into<String>,
        count: usize,
        append_comma: bool,
      ) -> String {
        format!(
          "{}{}{} {count}",
          if append_comma { ", " } else { "" },
          name.into(),
          ":".dimmed()
        )
      }

      let (_, appended_string) = Self::append_id_range(
        1..=self.total_workers,
        format!(
          "{}{}{}{}",
          "...".dimmed(),
          truncate_if_zero(
            make_workers_count_item(
              "idle".yellow().to_string(),
              idle_workers,
              false
            ),
            idle_workers
          ),
          truncate_if_zero(
            make_workers_count_item(
              "processing".blue().to_string(),
              processing_workers,
              true
            ),
            processing_workers
          ),
          truncate_if_zero(
            make_workers_count_item(
              "done".green().to_string(),
              done_workers,
              true
            ),
            done_workers
          ),
        )
        .dimmed()
        .to_string(),
      );

      eprintln!("{}", appended_string);

      return processing_workers + done_workers;
    }

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
    terminal_width: u16,
  ) {
    let tries_width = self.tries_string.len();

    let possible_bar_width = {
      if terminal_width < tries_width as u16
        || terminal_width + (tries_width as u16) < 45
      {
        DEFAULT_TERMINAL_WIDTH
      } else {
        terminal_width - tries_width as u16 * 2 - 55
      }
    };

    let bar_width = if DEFAULT_TERMINAL_WIDTH > possible_bar_width {
      possible_bar_width
    } else {
      DEFAULT_TERMINAL_WIDTH
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

    let size = self.term.size_checked();

    let (height, width) = match size {
      Some((height, width)) => (height, width),
      None => (DEFAULT_TERMINAL_HEIGHT, DEFAULT_TERMINAL_WIDTH),
    };

    let current_total = self.render_workers(&mut itoa_buf, progresses, height);

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
      width,
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
