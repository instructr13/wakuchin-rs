use console::style;

use wakuchin::convert::chars_to_wakuchin;
use wakuchin::result::Hit;

pub fn hit<F>(tries: usize) -> impl Fn(&Hit) {
  move |hit| {
    eprintln!(
      "{} {hit_on} {hit_chars}",
      style("hit").blue().underlined(),
      hit_on = style(format_args!(
        "{:<max_width$}",
        hit.hit_on,
        max_width = tries.to_string().len()
      ))
      .bold(),
      hit_chars = chars_to_wakuchin(&hit.chars)
    );
  }
}
