use crossterm::style::{Attribute, Stylize};

use wakuchin_core::convert::chars_to_wakuchin;
use wakuchin_core::result::Hit;

pub fn hit<F>(tries: usize) -> impl Fn(&Hit) {
  move |hit| {
    println!(
      "{} {bold_start}{hit_on:<hit_on_max_width$}{bold_end} {hit_chars}",
      "hit".blue().underlined(),
      bold_start = Attribute::Bold,
      bold_end = Attribute::Reset,
      hit_on = hit.hit_on,
      hit_on_max_width = tries.to_string().len(),
      hit_chars = chars_to_wakuchin(&hit.chars)
    );
  }
}
