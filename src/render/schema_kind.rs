//! The kinds of document of the schema, `reflect`, `bill`, `export` and
//! `doctor`, with the columns of each kind's CSV in their order.

use crate::render::schema::Column;

/// What a document of the schema holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    /// The report of `reflect`.
    Reflect,
    /// The bill of `bill`.
    Bill,
    /// The entries of `export`.
    Export,
    /// The findings of `doctor`.
    Doctor,
}

impl SchemaKind {
    /// The kind's name, the `kind` of its JSON document.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Reflect => "reflect",
            Self::Bill => "bill",
            Self::Export => "export",
            Self::Doctor => "doctor",
        }
    }

    /// The columns of the kind's CSV, in their order; the columns of the
    /// previous period follow them, then the change columns.
    #[must_use]
    pub const fn columns(self) -> &'static [Column] {
        match self {
            Self::Reflect => REFLECT_COLUMNS,
            Self::Bill => BILL_COLUMNS,
            Self::Export => EXPORT_COLUMNS,
            Self::Doctor => DOCTOR_COLUMNS,
        }
    }

    /// The columns of the previous period in the kind's CSV, each named
    /// `previous_<column>`, in their order.
    #[must_use]
    pub const fn previous_columns(self) -> &'static [Column] {
        match self {
            Self::Reflect => REFLECT_PREVIOUS_COLUMNS,
            Self::Bill | Self::Export | Self::Doctor => &[],
        }
    }

    /// The change columns of the kind's CSV, after the previous ones.
    #[must_use]
    pub const fn change_columns(self) -> &'static [Column] {
        match self {
            Self::Reflect => REFLECT_CHANGE_COLUMNS,
            Self::Bill | Self::Export | Self::Doctor => &[],
        }
    }
}

/// The columns of `reflect` that its rows set for the previous period.
const REFLECT_PREVIOUS_COLUMNS: &[Column] = &[
    Column::Key,
    Column::Id,
    Column::Name,
    Column::Date,
    Column::Start,
    Column::End,
    Column::Count,
    Column::NetSeconds,
    Column::Net,
    Column::HeldSeconds,
    Column::Held,
    Column::PausedSeconds,
    Column::Paused,
    Column::Amount,
    Column::Currency,
];

/// The change columns of `reflect`.
const REFLECT_CHANGE_COLUMNS: &[Column] = &[
    Column::ChangeCount,
    Column::ChangeNetSeconds,
    Column::ChangeNet,
    Column::ChangeHeldSeconds,
    Column::ChangeHeld,
    Column::ChangePausedSeconds,
    Column::ChangePaused,
    Column::ChangeAmount,
    Column::ChangePercent,
];

/// The columns of `reflect`.
const REFLECT_COLUMNS: &[Column] = &[
    Column::Part,
    Column::Group,
    Column::Key,
    Column::Id,
    Column::Short,
    Column::Name,
    Column::Client,
    Column::Date,
    Column::Start,
    Column::End,
    Column::Count,
    Column::NetSeconds,
    Column::Net,
    Column::Percent,
    Column::TargetSeconds,
    Column::Target,
    Column::BudgetSeconds,
    Column::Budget,
    Column::BilledSeconds,
    Column::Billed,
    Column::HeldSeconds,
    Column::Held,
    Column::PausedSeconds,
    Column::Paused,
    Column::Amount,
    Column::Currency,
    Column::Reason,
    Column::Note,
    Column::OtherId,
    Column::OtherShort,
    Column::Running,
];

/// The columns of `bill`.
const BILL_COLUMNS: &[Column] = &[
    Column::Part,
    Column::Group,
    Column::Key,
    Column::Id,
    Column::Short,
    Column::Name,
    Column::Client,
    Column::Date,
    Column::Start,
    Column::End,
    Column::Period,
    Column::Count,
    Column::NetSeconds,
    Column::Net,
    Column::Percent,
    Column::BilledSeconds,
    Column::Billed,
    Column::GrossSeconds,
    Column::Gross,
    Column::HeldSeconds,
    Column::Held,
    Column::Rate,
    Column::Amount,
    Column::Currency,
    Column::Note,
];

/// The columns of `export`.
const EXPORT_COLUMNS: &[Column] = &[
    Column::Id,
    Column::Short,
    Column::Project,
    Column::ProjectId,
    Column::Client,
    Column::Role,
    Column::Start,
    Column::End,
    Column::NetSeconds,
    Column::Net,
    Column::GrossSeconds,
    Column::Gross,
    Column::HeldSeconds,
    Column::Held,
    Column::PausedSeconds,
    Column::Paused,
    Column::Rate,
    Column::Amount,
    Column::Currency,
    Column::Note,
    Column::Tags,
    Column::DeterminedBy,
    Column::Billable,
    Column::Invoice,
    Column::Running,
];

/// The columns of `doctor`.
const DOCTOR_COLUMNS: &[Column] = &[
    Column::Severity,
    Column::Area,
    Column::Message,
    Column::File,
    Column::Line,
    Column::Entry,
    Column::Fix,
];
