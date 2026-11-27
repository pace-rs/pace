//! The structure checks of `pace doctor`: billable projects without a
//! client, and weekly targets above the 168 hours of a week.

use pace_time::format_duration;

use crate::{
    Catalog,
    doctor::{DataArea, DataFinding},
    report::{WEEK, targets_beyond_week},
};

/// A warning for each billable project without a client, by key, and for
/// weekly targets of the roles in use that add up to more than a week.
///
/// The refusals of the catalog, such as a project whose client doesn't
/// exist, come from [`Catalog::collecting`]; the app turns them into
/// findings with their file and line.
#[must_use]
pub fn structure_findings(catalog: &Catalog) -> Vec<DataFinding> {
    let mut findings: Vec<DataFinding> = catalog
        .projects()
        .filter(|project| project.billable && project.client.is_none())
        .map(|project| {
            DataFinding::warning(
                DataArea::Structure,
                format!(
                    "the project `{}` is billable, but has no client",
                    project.key
                ),
                format!(
                    "give it a client, or make it not billable, with `pace project edit {}`",
                    project.key
                ),
            )
        })
        .collect();
    findings.extend(targets_beyond_week(catalog).map(|total| {
        DataFinding::warning(
            DataArea::Structure,
            format!(
                "the weekly targets of the roles in use add up to {}, more than the {} of a week",
                format_duration(total),
                format_duration(WEEK)
            ),
            "lower a target with `pace role edit`".to_owned(),
        )
    }));
    findings
}
