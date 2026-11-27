//! The protection of billed entries: without `--change-billed`, the rules
//! refuse to adjust or forget a billed entry, and to undo a batch that
//! changed one, except the batch that marked it.

use pace_error::{ErrorKind, PaceError, PaceResult};

use crate::{EntryId, State};

/// The error code of a change that [`refuse_billed_change`] refuses
/// because it touches a billed entry; `change_billed` lets it through.
pub const BILLED: &str = "pace::core::billed";

/// The change of a billed entry that [`refuse_billed_change`] refuses;
/// it gives the message and the hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BilledChange {
    /// `adjust`, and `edit`, which runs an adjustment.
    Adjust,
    /// `forget`.
    Forget,
    /// `undo` of a batch that changed the entry.
    Undo,
}

/// Refuse `change` if one of `entries` is billed ([`State::billing`]):
/// the error names each billed entry and its invoice, and the flag
/// `--change-billed`. An entry the state doesn't know counts as not
/// billed.
///
/// The rules call it, and an adapter may call it before it asks or opens
/// an editor, so the refusal comes first.
///
/// # Errors
///
/// [`BILLED`] if one of `entries` is billed.
pub fn refuse_billed_change(
    state: &State,
    entries: impl IntoIterator<Item = EntryId>,
    change: BilledChange,
) -> PaceResult<()> {
    let mut names = Vec::new();
    let mut invoices = Vec::new();
    for id in entries {
        let (Some(entry), Some(billing)) = (state.entry(id), state.billing(id)) else {
            continue;
        };
        names.push(state.entry_name(entry));
        if !invoices.contains(&billing.invoice) {
            invoices.push(billing.invoice.clone());
        }
    }
    if names.is_empty() {
        return Ok(());
    }
    let invoices: Vec<String> = invoices.iter().map(ToString::to_string).collect();
    let invoices = if invoices.len() == 1 {
        format!("invoice {}", listed(&invoices))
    } else {
        format!("invoices {}", listed(&invoices))
    };
    let (guidance, hint) = match change {
        BilledChange::Adjust => (
            "{entries} is billed on {invoices}",
            "add `--change-billed` to change it anyway; the invoice keeps its amounts",
        ),
        BilledChange::Forget => (
            "{entries} is billed on {invoices}",
            "add `--change-billed` to forget it anyway, and take it off the invoice",
        ),
        BilledChange::Undo => (
            "the undo would change {entries}, billed on {invoices}",
            "add `--change-billed` to undo it anyway",
        ),
    };
    Err(PaceError::new(ErrorKind::InvalidInput, guidance)
        .attach_context("entries", listed(&names))
        .attach_context("invoices", invoices)
        .attach_hint(hint)
        .attach_error_code(BILLED))
}

/// `a`, `a and b`, `a, b and c`.
fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_joins_last_with_and() {
        let items = |all: &[&str]| {
            all.iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };

        assert_eq!(listed(&items(&[])), "");
        assert_eq!(listed(&items(&["a"])), "a");
        assert_eq!(listed(&items(&["a", "b"])), "a and b");
        assert_eq!(listed(&items(&["a", "b", "c"])), "a, b and c");
    }
}
