//! Output drawn by code for the terminal: the canvas it fits, bars and
//! headings, and the report's table.

mod report_table;

use std::{ffi::OsString, io::IsTerminal as _};

pub use report_table::{ReportTable, nothing_tracked, report_table};

/// The width without a terminal, and on a terminal whose width pace can't
/// learn.
pub const DEFAULT_WIDTH: usize = 80;

/// The eighths of a block, from one eighth to a full block.
const EIGHTHS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Where output goes: how wide it may be, and whether it may use colour
/// and symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Canvas {
    /// The columns a line may fill.
    pub width: usize,
    /// Whether headings and bars get colour.
    pub colour: bool,
    /// Whether bars use block characters; else `#`.
    pub symbols: bool,
}

impl Canvas {
    /// The canvas of standard output for this run.
    #[must_use]
    pub fn detect(plain: bool) -> Self {
        let terminal = std::io::stdout().is_terminal();
        let measured = terminal_size::terminal_size().map(|(width, _)| usize::from(width.0));
        Self::of(plain, terminal.then_some(measured), |name: &str| {
            std::env::var_os(name)
        })
    }

    /// The canvas of a run with `--plain` or without, on a terminal of the
    /// measured width (`Some(None)` when the width is unknown) or without
    /// one (`None`), in the environment `env`.
    ///
    /// On a terminal, `COLUMNS` comes before the measured width, as for
    /// clap's help. Without a terminal, lines fit 80 columns and get no
    /// colour; `NO_COLOR` and `--plain` turn colour off as well, and
    /// `--plain` uses `#` for bars.
    #[must_use]
    pub fn of(
        plain: bool,
        terminal: Option<Option<usize>>,
        env: impl Fn(&str) -> Option<OsString>,
    ) -> Self {
        let no_color = env("NO_COLOR").is_some_and(|value| !value.is_empty());
        let width = terminal.map_or(DEFAULT_WIDTH, |measured| {
            env("COLUMNS")
                .and_then(|columns| columns.to_str()?.parse().ok())
                .or(measured)
                .unwrap_or(DEFAULT_WIDTH)
        });
        Self {
            width,
            colour: terminal.is_some() && !plain && !no_color,
            symbols: !plain,
        }
    }

    /// `text` as a heading: bold when the canvas has colour.
    #[must_use]
    pub fn heading(&self, text: &str) -> String {
        if self.colour {
            format!("\x1b[1m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }

    /// A bar of `value` against `largest`, which fills `width` columns:
    /// block characters in eighths, `#` in whole columns on a plain canvas.
    /// A value above zero shows at least the smallest mark.
    #[must_use]
    pub fn bar(&self, value: i64, largest: i64, width: usize) -> String {
        if value <= 0 || largest <= 0 || width == 0 {
            return String::new();
        }
        let value = value.min(largest).unsigned_abs();
        let largest = largest.unsigned_abs();
        let steps: u64 = if self.symbols { 8 } else { 1 };
        let total = u64::try_from(width)
            .unwrap_or(u64::MAX)
            .saturating_mul(steps);
        let filled = (u128::from(value) * u128::from(total) / u128::from(largest)).max(1);
        let filled = u64::try_from(filled).unwrap_or(total);
        let (whole, rest) = (filled / steps, filled % steps);
        let full = if self.symbols { EIGHTHS[7] } else { '#' };
        let mut bar: String = (0..whole).map(|_| full).collect();
        if rest > 0 {
            let index = usize::try_from(rest - 1).unwrap_or(0);
            bar.push(EIGHTHS[index]);
        }
        if self.colour {
            format!("\x1b[36m{bar}\x1b[0m")
        } else {
            bar
        }
    }

    /// `text` cut to `width` columns, its end marked by `…`, or by `~` on
    /// a plain canvas.
    #[must_use]
    pub fn fit(&self, text: &str, width: usize) -> String {
        if text.chars().count() <= width {
            return text.to_owned();
        }
        let mark = if self.symbols { '…' } else { '~' };
        let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
        cut.push(mark);
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        |name: &str| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    const PLAIN: Canvas = Canvas {
        width: 80,
        colour: false,
        symbols: false,
    };

    const BLOCKS: Canvas = Canvas {
        width: 80,
        colour: false,
        symbols: true,
    };

    #[test]
    fn without_terminal_canvas_is_80_columns_without_colour() {
        let canvas = Canvas::of(false, None, env(&[("COLUMNS", "200")]));

        assert_eq!(canvas, BLOCKS);
    }

    #[test]
    fn terminal_gives_its_width_and_colour() {
        let canvas = Canvas::of(false, Some(Some(120)), env(&[]));

        assert_eq!(canvas.width, 120);
        assert!(canvas.colour);
    }

    #[test]
    fn columns_come_before_measured_width() {
        let canvas = Canvas::of(false, Some(Some(120)), env(&[("COLUMNS", "60")]));

        assert_eq!(canvas.width, 60);
    }

    #[test]
    fn terminal_of_unknown_width_is_80_columns() {
        let canvas = Canvas::of(false, Some(None), env(&[("COLUMNS", "wide")]));

        assert_eq!(canvas.width, 80);
    }

    #[test]
    fn no_color_and_plain_turn_colour_off() {
        let no_color = Canvas::of(false, Some(Some(100)), env(&[("NO_COLOR", "1")]));
        let plain = Canvas::of(true, Some(Some(100)), env(&[]));

        assert!(!no_color.colour);
        assert!(no_color.symbols);
        assert!(!plain.colour);
        assert!(!plain.symbols);
    }

    #[test]
    fn empty_no_color_keeps_colour() {
        let canvas = Canvas::of(false, Some(Some(100)), env(&[("NO_COLOR", "")]));

        assert!(canvas.colour);
    }

    #[test]
    fn largest_value_fills_bar_and_others_scale() {
        assert_eq!(BLOCKS.bar(100, 100, 4), "████");
        assert_eq!(BLOCKS.bar(50, 100, 4), "██");
        assert_eq!(BLOCKS.bar(25, 100, 1), "▎");
        assert_eq!(BLOCKS.bar(1, 1000, 4), "▏");
        assert_eq!(BLOCKS.bar(0, 100, 4), "");
    }

    #[test]
    fn plain_bar_uses_hashes_in_whole_columns() {
        assert_eq!(PLAIN.bar(100, 100, 4), "####");
        assert_eq!(PLAIN.bar(60, 100, 4), "##");
        assert_eq!(PLAIN.bar(1, 1000, 4), "#");
    }

    #[test]
    fn coloured_canvas_colours_bars_and_headings() {
        let canvas = Canvas {
            colour: true,
            ..BLOCKS
        };

        assert_eq!(canvas.bar(1, 1, 1), "\x1b[36m█\x1b[0m");
        assert_eq!(canvas.heading("Time"), "\x1b[1mTime\x1b[0m");
        assert_eq!(BLOCKS.heading("Time"), "Time");
    }

    #[test]
    fn long_text_is_cut_with_mark() {
        assert_eq!(BLOCKS.fit("Webshop relaunch", 8), "Webshop…");
        assert_eq!(PLAIN.fit("Webshop relaunch", 8), "Webshop~");
        assert_eq!(BLOCKS.fit("Web", 8), "Web");
    }
}
