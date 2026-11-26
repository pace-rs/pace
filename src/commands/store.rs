//! `pace store`: how pace keeps its data.

use abscissa_core::{Command, Runnable};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::Split;

use crate::{
    application::{PACE_APP, exit_on_error},
    output,
    session::Session,
};

/// Look after how pace keeps its data.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum StoreCmd {
    /// Split this device's event files by another period.
    Split(SplitCmd),
}

/// `pace store split`
#[derive(clap::Args, Command, Debug)]
pub struct SplitCmd {
    #[arg(
        long,
        value_name = "PERIOD",
        help = "year, month, week or day; the setting storage.split if left out"
    )]
    by: Option<String>,
}

impl Runnable for SplitCmd {
    fn run(&self) {
        exit_on_error(self.execute());
    }
}

impl SplitCmd {
    fn execute(&self) -> PaceResult<()> {
        let context = PACE_APP.context()?;
        let split = match &self.by {
            Some(by) => by
                .parse::<Split>()
                .map_err(|error| error.attach_exit_code(ExitCode::Usage))?,
            None => context.settings.settings.split,
        };
        let session = Session::open(context)?;
        let resplit = session.resplit(split)?;
        if resplit.from == resplit.to {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "this device's events are already split by {split}",
            )
            .attach_context("split", split.to_string())
            .attach_hint("name another period with `--by`")
            .attach_exit_code(ExitCode::NothingToDo));
        }
        if resplit.events == 0 {
            println!(
                "This device has no events yet; it splits them by {split} from the first one."
            );
        } else {
            println!(
                "Split this device's {} by {} instead of {}, into {}.",
                output::count(resplit.events, "event"),
                resplit.to,
                resplit.from,
                output::count(resplit.files_after, "file")
            );
            println!(
                "Other devices keep their split; `pace store split --by {split}` on each of them matches it."
            );
        }
        Ok(())
    }
}
