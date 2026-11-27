//! The comparison of `reflect --compare` in words: the previous value and
//! the change beside a number, for the table and the rows, and the
//! highlights.

use jiff::SignedDuration;
use pace_core::{Catalog, Compared, HoldReport, Money, ReportComparison, ReportPeriod};
use pace_time::format_duration;

use crate::config::hold_source_name;
use crate::output::{counted, project_label};
use crate::render::{
    Canvas,
    report_table::{marked, period_days},
};

/// A difference of time with its sign: `+1h 5m`, `-30m`, `0s`.
#[must_use]
pub(crate) fn signed_duration(difference: SignedDuration) -> String {
    let sign = if difference.is_positive() { "+" } else { "" };
    format!("{sign}{}", format_duration(difference))
}

/// A difference of money with its sign: `+60.00`, `-12.50`, `0.00`.
#[must_use]
pub(crate) fn signed_money(difference: &Money) -> String {
    let sign = if difference.minor() > 0 { "+" } else { "" };
    format!("{sign}{difference}")
}

/// A change in percent with its sign: `+45%`, `-54%`, `0%`; nothing when
/// the previous value is zero.
#[must_use]
pub(crate) fn signed_percent(percent: Option<i64>) -> String {
    percent.map_or_else(String::new, |percent| {
        let sign = if percent > 0 { "+" } else { "" };
        format!("{sign}{percent}%")
    })
}

/// The cells beside a time: `was 3h`, `+2h 15m`, `+75%`.
#[must_use]
pub(crate) fn time_change_cells(change: &Compared<SignedDuration>) -> Vec<String> {
    vec![
        format!("was {}", format_duration(change.previous)),
        signed_duration(change.difference),
        signed_percent(change.percent),
    ]
}

/// The cells beside an amount: `was 120.00 EUR`, `+60.00`, `+50%`; empty
/// cells for amounts in two currencies, which never compare.
#[must_use]
pub(crate) fn money_change_cells(change: Option<&Compared<Money>>) -> Vec<String> {
    change.map_or_else(
        || vec![String::new(); 3],
        |change| {
            vec![
                format!(
                    "was {} {}",
                    change.previous,
                    change.previous.currency().code()
                ),
                signed_money(&change.difference),
                signed_percent(change.percent),
            ]
        },
    )
}

/// A time and its change in a sentence: `7h 15m (was 3h 30m, +3h 45m,
/// +107%)`.
fn time_with_change(current: SignedDuration, change: &Compared<SignedDuration>) -> String {
    let cells: Vec<String> = time_change_cells(change)
        .into_iter()
        .filter(|cell| !cell.is_empty())
        .collect();
    format!("{} ({})", format_duration(current), cells.join(", "))
}

/// `Compared with Mon 2026-11-16 to Sun 2026-11-22.`
#[must_use]
pub(crate) fn compared_with(previous: &ReportPeriod) -> String {
    format!("Compared with {}.", period_days(previous))
}

/// `Tracked 7h 15m (was 3h 30m, +3h 45m, +107%), untracked …`.
#[must_use]
pub(crate) fn compared_summary(
    tracked: SignedDuration,
    untracked: SignedDuration,
    comparison: &ReportComparison,
) -> String {
    format!(
        "Tracked {}, untracked {}.",
        time_with_change(tracked, &comparison.tracked),
        time_with_change(untracked, &comparison.untracked)
    )
}

/// The holds of the previous period in a sentence, its sources and the
/// reasons named above: `Previous period: 2 holds, 30m: inner 0, outer 2
/// (30m), unknown 0; reasons: phone call 1.`
#[must_use]
pub(crate) fn compared_holds(comparison: &ReportComparison, holds: &HoldReport) -> String {
    let total = &comparison.holds.total;
    let by_source: Vec<String> = holds
        .by_source
        .iter()
        .zip(&comparison.holds.by_source)
        .map(|(share, change)| {
            let name = hold_source_name(share.source);
            match change.count.previous {
                0 => format!("{name} 0"),
                count => format!("{name} {count} ({})", format_duration(change.time.previous)),
            }
        })
        .collect();
    let reasons: Vec<String> = holds
        .reasons
        .iter()
        .zip(&comparison.holds.reasons)
        .map(|(reason, change)| format!("{} {}", reason.reason, change.previous))
        .collect();
    let reasons = if reasons.is_empty() {
        String::new()
    } else {
        format!("; reasons: {}", reasons.join(", "))
    };
    format!(
        "Previous period: {}, {}: {}{reasons}.",
        counted(previous_count(total.count.previous), "hold"),
        format_duration(total.time.previous),
        by_source.join(", ")
    )
}

/// The pauses of the previous period, of the kinds named above:
/// `Previous period: lunch 2 (1h).`
#[must_use]
pub(crate) fn compared_pauses(comparison: &ReportComparison, kinds: &[&str]) -> String {
    let pauses: Vec<String> = kinds
        .iter()
        .zip(&comparison.pauses)
        .map(|(kind, change)| {
            format!(
                "{kind} {} ({})",
                change.count.previous,
                format_duration(change.time.previous)
            )
        })
        .collect();
    format!("Previous period: {}.", pauses.join(", "))
}

fn previous_count(count: i64) -> usize {
    usize::try_from(count).unwrap_or(0)
}

/// The busiest day and the top project of both periods, as sentences.
#[must_use]
pub(crate) fn highlights_part(
    comparison: &ReportComparison,
    catalog: &Catalog,
    canvas: Canvas,
) -> Vec<String> {
    let day = |highlights: &pace_core::Highlights| {
        highlights.busiest_day.map_or_else(
            || "none".to_owned(),
            |day| {
                format!(
                    "{}, {}",
                    day.date.strftime("%a %Y-%m-%d"),
                    marked(day.net, day.running)
                )
            },
        )
    };
    let project = |highlights: &pace_core::Highlights| {
        highlights.top_project.as_ref().map_or_else(
            || "none".to_owned(),
            |top| {
                format!(
                    "{}, {}",
                    project_label(&top.project, catalog),
                    marked(top.net, top.running)
                )
            },
        )
    };
    let (now, before) = (&comparison.highlights, &comparison.previous_highlights);
    let mut lines = vec![canvas.heading("Highlights")];
    for text in [
        format!(
            "Busiest day: {}; in the previous period {}.",
            day(now),
            day(before)
        ),
        format!(
            "Top project: {}; in the previous period {}.",
            project(now),
            project(before)
        ),
    ] {
        lines.extend(canvas.wrap(&text, "  "));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differences_carry_their_sign() {
        assert_eq!(signed_duration(SignedDuration::from_mins(65)), "+1h 5m");
        assert_eq!(signed_duration(SignedDuration::from_mins(-30)), "-30m");
        assert_eq!(signed_duration(SignedDuration::ZERO), "0s");
        assert_eq!(signed_percent(Some(45)), "+45%");
        assert_eq!(signed_percent(Some(-54)), "-54%");
        assert_eq!(signed_percent(Some(0)), "0%");
        assert_eq!(signed_percent(None), "");
        let eur = "EUR".parse().unwrap();
        assert_eq!(signed_money(&Money::from_minor(6000, eur)), "+60.00");
        assert_eq!(signed_money(&Money::from_minor(-1250, eur)), "-12.50");
    }

    #[test]
    fn amounts_in_two_currencies_leave_change_empty() {
        assert_eq!(money_change_cells(None), ["", "", ""]);
    }
}
