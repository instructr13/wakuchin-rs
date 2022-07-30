use std::io::stderr;
use std::time::Duration;

use crossterm::cursor::{MoveLeft, MoveUp};
use crossterm::style::{Attribute, Print, Stylize};
use crossterm::terminal::ClearType;
use crossterm::{execute, terminal};

use wakuchin::convert::chars_to_wakuchin;
use wakuchin::progress::{
  DoneDetail, HitCounter, ProcessingDetail, Progress, ProgressKind,
};

pub fn progress<F>(
  tries: usize,
  times: usize,
) -> impl Fn(&[Progress], &[HitCounter], Duration, usize, bool) + Copy {
  move |progresses, hit_counters, elapsed_time, current_diff, all_done| {
    let progress_bar_width = 33;
    let progress_len = progresses.len() + hit_counters.len() + 1;
    let elapsed_time = elapsed_time.as_secs_f32();
    let tries_width = tries.to_string().len();
    let bold_start = Attribute::Bold;
    let bold_end = Attribute::Reset;

    let mut current_total = 0;
    let mut current_hit_total = 0;

    for hit_counter in hit_counters {
      let chars = chars_to_wakuchin(&hit_counter.chars).grey();
      let count = hit_counter.hits;

      current_hit_total += count;

      println!(
        "        {} {chars}: {bold_start}{count:<tries_width$}{bold_end} ({:.3}%)",
        "hits".blue().underlined(),
        count as f64 / tries as f64 * 100.0,
      );
    }

    println!(
      "  {} {bold_start}{current_hit_total:<tries_width$}{bold_end} / {tries} ({:.3}%)",
      "total hits".blue().underlined(),
      current_hit_total as f64 / tries as f64 * 100.0,
    );

    for progress in progresses {
      match progress {
        Progress(ProgressKind::Idle(0, 1)) => {
          println!("{}", "Idle".yellow());
        }
        Progress(ProgressKind::Idle(id, total)) => {
          let id_width = total.to_string().len();

          println!(
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

              println!(
                "{} {} • {current:<tries_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).dim(),
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

              println!(
                "{bold_start}#{id:<id_width$}{bold_end} {} {} • {current:<tries_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).dim(),
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

          println!(
            "{} {}",
            "Done      ".green(),
            " ".repeat(times * 8 + tries.to_string().len() * 2 + 5),
          );
        }
        Progress(ProgressKind::Done(DoneDetail {
          id,
          total,
          total_workers,
        })) => {
          current_total += total;

          let id_width = total_workers.to_string().len();

          println!(
            "{bold_start}#{id:<id_width$}{bold_end} {} {}",
            "Done      ".green(),
            " ".repeat(times * 8 + tries.to_string().len() * 2 + 5),
          );
        }
      }
    }

    if all_done {
      execute!(
        stderr(),
        Print("Status ".bold()),
        Print("All Done".bold().green())
      )
      .unwrap();

      for _ in 0..progress_len {
        execute!(
          stderr(),
          terminal::Clear(ClearType::CurrentLine),
          MoveUp(1),
          terminal::Clear(ClearType::CurrentLine)
        )
        .unwrap();
      }
    } else {
      let width = terminal::size().unwrap().0 - tries_width as u16 * 2 - 55;

      let mut progress = String::new();

      let progress_size = if progress_bar_width > width {
        width
      } else {
        progress_bar_width
      };

      let progress_percentage = current_total as f64 / tries as f64 * 100.0;
      let progress_rate = current_diff as f32 / elapsed_time;
      let progress_remaining_time =
        (tries - current_total) as f32 / progress_rate;

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
          "{progress} • {}: {bold_start}{current_total:<tries_width$}{bold_end} / {tries} ({progress_percentage:.0}%, {progress_rate}/sec, eta: {progress_remaining_time:>3.0}sec)   ",
          "total".green().underlined(),
          progress_rate = human_format::Formatter::new().format(progress_rate.into()),
        )),
        MoveLeft(u16::MAX),
        MoveUp(progress_len as u16)
      )
      .unwrap();
    }
  }
}
