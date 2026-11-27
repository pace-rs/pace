//! The data checks of `pace doctor`: the entries, the structure and
//! billing, run by `pace_core` on the events and the catalog that the file
//! checks read.

use std::ffi::OsString;

use jiff::{ZonedDateTime, tz::TimeZone};
use pace_core::{
    DataFinding, DoctorInput, State, billing_findings, entry_findings, structure_findings,
};
use pace_error::PaceResult;

use crate::clock::{clock_for_run, zone};
use crate::config::Settings;
use crate::doctor::{FileChecks, Finding, FindingArea, FindingSeverity};

/// Every finding of the entries, the structure and billing, with
/// `test_now` as the value of `PACE_TEST_NOW`: the entries, the
/// catalog's refusals, the other structure checks, then billing.
///
/// The checks need the catalog, so they run only if every catalog file
/// reads, and so do the catalog's refusals; the file findings name the
/// mistakes otherwise. An entry's day is its start day in the zone of
/// `reports.time-zone`, as for `pace bill`.
#[must_use]
pub fn data_check_findings(files: &FileChecks, test_now: Option<OsString>) -> Vec<Finding> {
    let Some(catalog) = &files.catalog else {
        return Vec::new();
    };
    let settings = files
        .settings
        .as_ref()
        .map_or_else(Settings::default, |loaded| loaded.settings.clone());
    let (zone, now) = match reports_now(&settings, test_now) {
        Ok(moment) => moment,
        Err(error) => {
            let mut findings = vec![Finding::from_error(
                FindingSeverity::Error,
                FindingArea::Files,
                &error,
            )];
            findings.extend(files.refusals.iter().cloned());
            return findings;
        }
    };
    let state = State::from_events(&files.events).resolve_projects(catalog);
    let input = DoctorInput {
        state: &state,
        catalog,
        events: &files.events,
        zone: &zone,
        now: &now,
    };
    let mut findings = found(entry_findings(&input));
    findings.extend(files.refusals.iter().cloned());
    findings.extend(found(structure_findings(catalog)));
    findings.extend(found(billing_findings(&input)));
    findings
}

/// The zone of `reports.time-zone` and now in it, by the clock of this
/// run.
fn reports_now(
    settings: &Settings,
    test_now: Option<OsString>,
) -> PaceResult<(TimeZone, ZonedDateTime)> {
    let zone = zone(&settings.reports_time_zone)?;
    let now = clock_for_run(&settings.time_zone, test_now)?
        .now()
        .with_time_zone(zone.clone());
    Ok((zone, now))
}

/// The findings of the app for the data findings of `pace_core`.
fn found(findings: Vec<DataFinding>) -> Vec<Finding> {
    findings
        .into_iter()
        .map(|finding| Finding {
            severity: finding.severity,
            area: finding.area.into(),
            message: finding.message,
            file: None,
            line: None,
            entry: finding.entry,
            fix: finding.fix,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_catalog_no_data_check_runs() {
        let refusal = Finding::new(FindingSeverity::Error, FindingArea::Structure, "unknown");
        let files = FileChecks {
            refusals: vec![refusal],
            ..FileChecks::default()
        };

        assert_eq!(data_check_findings(&files, None), []);
    }

    #[test]
    fn data_finding_keeps_its_entry_and_fix_in_its_area() {
        let finding = DataFinding {
            severity: FindingSeverity::Warning,
            area: pace_core::DataArea::Billing,
            message: "differs".to_owned(),
            entry: Some("ad5e".to_owned()),
            fix: Some("look".to_owned()),
        };

        let found = found(vec![finding]);

        assert_eq!(found[0].area, FindingArea::Billing);
        assert_eq!(found[0].entry.as_deref(), Some("ad5e"));
        assert_eq!(found[0].fix.as_deref(), Some("look"));
    }
}
