//! `pace doctor`: check the directories, the files, the device and setup,
//! and say how to fix each problem; it changes nothing.

use abscissa_core::{Command, Runnable};
use pace_error::{ExitCode, PaceResult};

use crate::application::{Context, PACE_APP, exit_on_error, exit_with};
use crate::doctor::{check_everything, doctor_lines, doctor_rows, has_errors};
use crate::print::print_lines;
use crate::render::schema::{SchemaKind, schema_json};

/// `pace doctor`
#[derive(clap::Args, Command, Debug)]
pub struct DoctorCmd {
    #[arg(long, help = "Print the findings as JSON, in the schema of version 1")]
    json: bool,
}

impl Runnable for DoctorCmd {
    fn run(&self) {
        match PACE_APP.context().and_then(|context| self.execute(context)) {
            Ok(true) => exit_with(ExitCode::Error),
            Ok(false) => {}
            Err(error) => exit_on_error(Err(error)),
        }
    }
}

impl DoctorCmd {
    /// Run every check and print the findings; whether one is an error.
    fn execute(&self, context: &Context) -> PaceResult<bool> {
        let findings = check_everything(&context.dirs, &context.cwd);
        if self.json {
            print_lines([schema_json(SchemaKind::Doctor, &doctor_rows(&findings))?])?;
        } else {
            print_lines(doctor_lines(&findings))?;
        }
        Ok(has_errors(&findings))
    }
}
