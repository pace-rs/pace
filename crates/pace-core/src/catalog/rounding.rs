//! Rounding: the client's rule that rounds billable time to a step, per
//! entry or per day and project; no rounding is the default.

use jiff::SignedDuration;
use pace_error::{ErrorKind, PaceError, PaceResult};

keyword_enum!(
    /// What a client's rounding rounds.
    RoundingPer, "rounding unit", {
        /// Each entry by itself.
        Entry => "entry",
        /// The time of a day and project together.
        Day => "day",
    }
);

keyword_enum!(
    /// Which way a client's rounding goes.
    RoundingMode, "rounding mode", {
        /// Up to the next step.
        Up => "up",
        /// Down to the step before.
        Down => "down",
        /// To the nearest step; half a step rounds up.
        Nearest => "nearest",
    }
);

/// A client's rule for billable time: a step such as 15 minutes, a
/// [`RoundingMode`] and a [`RoundingPer`]. The default, [`Rounding::NONE`],
/// keeps every duration as it is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rounding(Option<Rule>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rule {
    per: RoundingPer,
    step: SignedDuration,
    mode: RoundingMode,
}

impl Rounding {
    /// No rounding: every duration stays as it is.
    pub const NONE: Self = Self(None);

    /// Round to `step` in `mode`, per entry or per day.
    ///
    /// # Errors
    ///
    /// If `step` is not longer than zero.
    pub fn new(per: RoundingPer, step: SignedDuration, mode: RoundingMode) -> PaceResult<Self> {
        if step.is_positive() {
            Ok(Self(Some(Rule { per, step, mode })))
        } else {
            Err(PaceError::new(
                ErrorKind::InvalidInput,
                "pace can't round to a step of {step}",
            )
            .attach_context("step", format!("{step:#}"))
            .attach_hint("give the rounding a step longer than zero, like `15m`")
            .attach_error_code("pace::core::rounding-step"))
        }
    }

    /// What it rounds; `None` without rounding.
    #[must_use]
    pub fn per(&self) -> Option<RoundingPer> {
        self.0.map(|rule| rule.per)
    }

    /// The step it rounds to; `None` without rounding.
    #[must_use]
    pub fn step(&self) -> Option<SignedDuration> {
        self.0.map(|rule| rule.step)
    }

    /// Which way it rounds; `None` without rounding.
    #[must_use]
    pub fn mode(&self) -> Option<RoundingMode> {
        self.0.map(|rule| rule.mode)
    }

    /// `duration` rounded by the rule: with a step of 15 minutes, 46 minutes
    /// round up to 60, down to 45 and to the nearest at 45. The result
    /// saturates at the limits of [`SignedDuration`].
    #[must_use]
    pub fn round(&self, duration: SignedDuration) -> SignedDuration {
        let Some(rule) = self.0 else {
            return duration;
        };
        let nanos = duration.as_nanos();
        let step = rule.step.as_nanos();
        let below = nanos - nanos.rem_euclid(step);
        let rest = nanos - below;
        let rounded = match rule.mode {
            RoundingMode::Down => below,
            RoundingMode::Up if rest == 0 => below,
            RoundingMode::Nearest if rest * 2 < step => below,
            RoundingMode::Up | RoundingMode::Nearest => below + step,
        };
        SignedDuration::try_from_nanos_i128(rounded).unwrap_or(if rounded < 0 {
            SignedDuration::MIN
        } else {
            SignedDuration::MAX
        })
    }
}
