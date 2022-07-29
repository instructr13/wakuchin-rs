use std::io::stderr;

use crossterm::cursor::MoveUp;
use crossterm::style::{Attribute, Stylize};
use crossterm::terminal::ClearType;
use crossterm::{execute, terminal};

use wakuchin::convert::chars_to_wakuchin;
use wakuchin::progress::{
  HitCounter, ProcessingDetail, Progress, ProgressKind,
};

pub fn progress<F>(
  tries: usize,
  times: usize,
) -> impl Fn(&[Progress], &[HitCounter], bool) + Copy {
  move |progresses, hit_counters, all_done| {
    let progress_len = progresses.len() + hit_counters.len();

    for hit_counter in hit_counters {
      let chars = chars_to_wakuchin(&hit_counter.chars).grey();
      let count = hit_counter.hits;
      let count_max_width = tries.to_string().len();

      println!(
        "    {} {chars}: {bold_start}{count:<count_max_width$}{bold_end} ({:.3}%)",
        "hit".blue().underlined(),
        count as f64 / tries as f64 * 100.0,
        bold_start = Attribute::Bold,
        bold_end = Attribute::Reset,
      );
    }

    for progress in progresses {
      /*
      execute!(stderr(), terminal::Clear(ClearType::CurrentLine))
        .expect("terminal initialization failed");
        */

      match progress {
        Progress(ProgressKind::Idle(0, 1)) => {
          println!("{}", "Idle".yellow());
        }
        Progress(ProgressKind::Idle(id, total)) => {
          println!(
            "{bold_start}#{id:<id_width$}{bold_end} {}",
            "Idle".yellow(),
            bold_start = Attribute::Bold,
            id_width = total.to_string().len(),
            bold_end = Attribute::Reset,
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
              println!(
                "{} {} • {current:<current_max_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).grey(),
                current_max_width = tries.to_string().len(),
              );
            }
            ProcessingDetail {
              id,
              current,
              total,
              total_workers,
              ..
            } => {
              println!(
                "{bold_start}#{id:<id_width$}{bold_end} {} {} • {current:<current_max_width$} / {total}",
                "Processing".blue(),
                chars_to_wakuchin(&processing_detail.wakuchin).grey(),
                bold_start = Attribute::Bold,
                id_width = total_workers.to_string().len(),
                bold_end = Attribute::Reset,
                current_max_width = tries.to_string().len(),
              );
            }
          }
        }
        Progress(ProgressKind::Done(0, 1)) => {
          println!(
            "{} {}",
            "Done      ".green(),
            " ".repeat(times * 8 + tries.to_string().len() * 2 + 5),
          );
        }
        Progress(ProgressKind::Done(id, total)) => {
          let id_width = total.to_string().len();

          println!(
            "{bold_start}#{id:<id_width$}{bold_end} {} {}",
            "Done      ".green(),
            " ".repeat(times * 8 + tries.to_string().len() * 2 + 5),
            bold_start = Attribute::Bold,
            bold_end = Attribute::Reset,
          );
        }
      }
    }

    if all_done {
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
      execute!(stderr(), MoveUp(progress_len as u16)).unwrap();
    }
  }
}
