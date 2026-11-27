//! The device and setup checks of `pace doctor`: a device ID that another
//! machine writes with too, and a `pace setup` that waits to be continued.

use std::path::Path;

use pace_core::{DeviceId, Hlc, RecordedEvent};
use pace_store::{detect_cloned_device, existing_device_id, last_written_hlc};

use crate::doctor::{Finding, FindingArea, FindingSeverity};
use crate::setup::setup_file::setup_file_in;

/// The latest HLC of the events of `device` among `events`: how far its
/// own log reaches.
fn own_last(events: &[RecordedEvent], device: DeviceId) -> Option<Hlc> {
    events
        .iter()
        .filter(|event| event.device == device)
        .map(|event| event.hlc)
        .max()
}

/// The finding of a cloned device ID, or of a machine file that doesn't
/// read.
///
/// The ID is cloned when the own log of this machine's ID among `events`
/// reaches beyond what the machine directory `machine` remembers writing:
/// another machine writes with the same ID. The machine's `device.toml`
/// or `cursors.toml` may not read either. Before the machine's first run,
/// there is nothing to check.
#[must_use]
pub fn device_findings(machine: &Path, events: &[RecordedEvent]) -> Vec<Finding> {
    let device_error = |error: &pace_error::PaceError| {
        vec![Finding::from_error(
            FindingSeverity::Error,
            FindingArea::Device,
            error,
        )]
    };
    let device = match existing_device_id(machine) {
        Ok(Some(device)) => device,
        Ok(None) => return Vec::new(),
        Err(error) => return device_error(&error),
    };
    let remembered = match last_written_hlc(machine) {
        Ok(remembered) => remembered,
        Err(error) => return device_error(&error),
    };
    detect_cloned_device(machine, own_last(events, device), remembered)
        .map_or_else(Vec::new, |error| device_error(&error))
}

/// The warning of an interrupted `pace setup`, if its `setup.toml` waits
/// in the machine directory `machine`.
#[must_use]
pub fn unfinished_setup(machine: &Path) -> Option<Finding> {
    let path = setup_file_in(machine);
    path.is_file().then(|| {
        Finding::new(
            FindingSeverity::Warning,
            FindingArea::Setup,
            "a setup was interrupted and waits to be continued",
        )
        .in_file(&path)
        .with_fix("continue with `pace setup`")
    })
}

#[cfg(test)]
mod tests {
    use pace_store::testing::scratch_dir;

    use super::*;

    #[test]
    fn machine_before_its_first_run_has_no_device_finding() -> Result<(), std::io::Error> {
        let machine = scratch_dir()?;

        assert_eq!(device_findings(machine.path(), &[]), []);
        Ok(())
    }

    #[test]
    fn broken_device_file_is_device_finding() -> Result<(), std::io::Error> {
        let machine = scratch_dir()?;
        std::fs::write(machine.path().join("device.toml"), "version = 1\nid = 7\n")?;

        let findings = device_findings(machine.path(), &[]);

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].area, FindingArea::Device);
        assert!(findings[0].fix.is_some());
        Ok(())
    }

    #[test]
    fn setup_file_in_machine_directory_is_an_unfinished_setup() -> Result<(), std::io::Error> {
        let machine = scratch_dir()?;
        assert_eq!(unfinished_setup(machine.path()), None);

        std::fs::write(setup_file_in(machine.path()), "version = 1\n")?;
        let finding = unfinished_setup(machine.path());

        assert_eq!(
            finding.and_then(|finding| finding.fix),
            Some("continue with `pace setup`".to_owned())
        );
        Ok(())
    }
}
