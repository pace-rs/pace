//! `pace client`: add, list and edit the clients in `clients.toml`.

use std::path::Path;

use abscissa_core::{Command, Runnable};
use jiff::civil::Date;
use pace_core::{Catalog, Client, ClientKey, Rounding, RoundingMode};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult, closest_name};
use pace_store::{
    catalog::{CLIENTS_FILE, read_catalog_with},
    field_edit::{FieldEdit, apply_field_edits, table_text, write_field_edits},
};

use crate::application::{Context, PACE_APP, exit_on_error, print_error};
use crate::clock::clock_for_run;
use crate::editor::{EditedFile, edit_in_editor, edit_until_checked, editor_command};
use crate::output::{aligned, listed, usage};
use crate::print::print_lines;
use crate::prompt::Ask;
use crate::questions::add_client::{AddClientFlow, ClientFlags};
use crate::questions::{Asked, ask_flow};

/// Keep the clients: who gets billed, in which currency, at which rates.
#[derive(clap::Subcommand, Command, Debug, Runnable)]
pub enum ClientCmd {
    /// Add a client; pace asks for what the flags leave open
    Add(ClientAddCmd),

    /// Show the clients with the rate of today
    List(ClientListCmd),

    /// Change a client's table in your editor
    Edit(ClientEditCmd),
}

/// `pace client add`
#[derive(clap::Args, Command, Debug)]
pub struct ClientAddCmd {
    #[arg(
        value_name = "KEY",
        help = "What you type for the client, such as `acme`"
    )]
    key: String,

    #[arg(long, help = "What a bill shows; the key if left out")]
    name: Option<String>,

    #[arg(
        long,
        value_name = "CODE",
        help = "The ISO 4217 currency of its rates and bills, such as EUR"
    )]
    currency: Option<String>,

    #[arg(
        long,
        value_name = "AMOUNT",
        help = "The hourly rate, such as 95.50; none if left out"
    )]
    rate: Option<String>,

    #[arg(
        long,
        value_name = "DAY",
        requires = "rate",
        help = "The first day the rate holds, such as 2026-11-01; today if left out"
    )]
    from: Option<String>,

    #[arg(
        long,
        value_name = "DURATION",
        help = "Round billable time to this step, such as 15m; no rounding if left out"
    )]
    rounding_step: Option<String>,

    #[arg(
        long,
        value_parser = ["up", "down", "nearest"],
        requires = "rounding_step",
        help = "up, down or nearest; up if left out"
    )]
    rounding_mode: Option<String>,

    #[arg(
        long,
        value_parser = ["entry", "day"],
        requires = "rounding_step",
        help = "Round each entry, or the time of a day and project; entry if left out"
    )]
    rounding_per: Option<String>,
}

impl Runnable for ClientAddCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ClientAddCmd {
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = client_key(&self.key)?;
        let flags = ClientFlags {
            name: self.name.clone(),
            currency: self.currency.clone(),
            rate: self.rate.clone(),
            from: self.from.clone(),
            rounding_step: self.rounding_step.clone(),
            rounding_mode: self
                .rounding_mode
                .as_deref()
                .map(str::parse)
                .transpose()
                .map_err(usage)?,
            rounding_per: self
                .rounding_per
                .as_deref()
                .map(str::parse)
                .transpose()
                .map_err(usage)?,
        };
        let clients_file = context.dirs.data.path.join(CLIENTS_FILE);
        let flow =
            AddClientFlow::new(&context.catalog, &clients_file, key, today(context)?, flags)?;
        let asked = ask_flow(flow, &context.terminal)?;
        let lines = added_lines(&asked);
        write_field_edits(asked.edits)?;
        print_lines(lines)
    }
}

/// Today in the user's time zone.
fn today(context: &Context) -> PaceResult<Date> {
    let clock = clock_for_run(
        &context.settings.settings.time_zone,
        context.test_now.clone(),
    )?;
    Ok(clock.now().date())
}

/// What `pace client add` says it did, and the defaults it took.
fn added_lines(asked: &Asked<Client>) -> Vec<String> {
    let client = &asked.made;
    let rate = client.rates.iter().next().map_or_else(String::new, |rate| {
        format!(" at {} an hour from {}", rate.hourly, rate.from)
    });
    let rounding = rounding_words(client.rounding)
        .map_or_else(String::new, |rounding| format!(", rounding {rounding}"));
    let mut lines = vec![format!(
        "Added the client `{}` ({}) in {}{rate}{rounding}.",
        client.key, client.name, client.currency
    )];
    if !asked.defaults.is_empty() {
        lines.push(format!(
            "pace took these defaults: {}; `pace client edit {}` changes them.",
            listed(&asked.defaults),
            client.key
        ));
    }
    lines
}

/// `rounding` in words: `up to 15m per entry`; `None` without rounding.
fn rounding_words(rounding: Rounding) -> Option<String> {
    let (per, step, mode) = (rounding.per()?, rounding.step()?, rounding.mode()?);
    let way = match mode {
        RoundingMode::Up => "up to",
        RoundingMode::Down => "down to",
        RoundingMode::Nearest => "to the nearest",
    };
    Some(format!("{way} {step:#} per {}", per.as_str()))
}

/// `pace client list`
#[derive(clap::Args, Command, Debug)]
pub struct ClientListCmd {}

impl Runnable for ClientListCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| {
            let today = today(context)?;
            print_lines(client_lines(&context.catalog, today))
        }));
    }
}

/// The clients of `catalog` as a table: key, name, currency, the rate
/// that holds on `today` and the rounding.
fn client_lines(catalog: &Catalog, today: Date) -> Vec<String> {
    let clients: Vec<&Client> = catalog.clients().collect();
    if clients.is_empty() {
        return vec!["There are no clients yet; `pace client add <key>` adds one.".to_owned()];
    }
    let mut rows = vec![
        ["key", "name", "currency", "rate today", "rounding"]
            .map(str::to_owned)
            .to_vec(),
    ];
    for client in clients {
        rows.push(vec![
            client.key.to_string(),
            client.name.clone(),
            client.currency.to_string(),
            client
                .rates
                .on(today)
                .map_or_else(|| "no rate yet".to_owned(), |rate| rate.hourly.to_string()),
            rounding_words(client.rounding).unwrap_or_else(|| "none".to_owned()),
        ]);
    }
    aligned(&rows)
}

/// `pace client edit`
#[derive(clap::Args, Command, Debug)]
pub struct ClientEditCmd {
    #[arg(value_name = "KEY", help = "The client, by its key")]
    key: String,
}

impl Runnable for ClientEditCmd {
    fn run(&self) {
        exit_on_error(PACE_APP.context().and_then(|context| self.execute(context)));
    }
}

impl ClientEditCmd {
    /// Open the client's table in the editor until the catalog reads the
    /// clients file with it, then write it in its place.
    fn execute(&self, context: &Context) -> PaceResult<()> {
        let key = client_key(&self.key)?;
        let client = known_client(&context.catalog, &key)?;
        let home = &context.dirs.data.path;
        let asker = &context.terminal;
        if !asker.can_ask() && context.editor.is_none() {
            return Err(no_terminal_to_edit(&home.join(CLIENTS_FILE), &key));
        }
        let editor = editor_command(context.editor.as_deref(), asker.can_ask())?;
        let line = edit_client_table(
            home,
            client,
            asker,
            |text| edit_in_editor(&editor, text, ".toml"),
            print_error,
        )?;
        print_lines([line])
    }
}

/// The client `key` of `catalog`.
///
/// # Errors
///
/// If `catalog` has no client `key`: the error names the closest key.
fn known_client<'catalog>(
    catalog: &'catalog Catalog,
    key: &ClientKey,
) -> PaceResult<&'catalog Client> {
    catalog.client(key).ok_or_else(|| {
        let hint = closest_name(
            key.as_str(),
            catalog.clients().map(|client| client.key.as_str()),
        )
        .map_or_else(
            || "`pace client list` shows the clients".to_owned(),
            |closest| format!("did you mean `{closest}`?"),
        );
        PaceError::new(
            ErrorKind::InvalidInput,
            "pace doesn't know the client `{key}`",
        )
        .attach_context("key", key.to_string())
        .attach_hint(hint)
    })
}

/// Let the user change the table of `client` in the clients file of
/// `home` with `edit` until the catalog reads the file with it, and write
/// it in its place; the sentence that says so. A mistake is shown with
/// `report`, and `asker` offers to open the table again.
fn edit_client_table(
    home: &Path,
    client: &Client,
    asker: &dyn Ask,
    edit: impl FnMut(&str) -> PaceResult<String>,
    report: impl Fn(&PaceError),
) -> PaceResult<String> {
    let clients_file = home.join(CLIENTS_FILE);
    let key = client.key.as_str();
    let text = std::fs::read_to_string(&clients_file).map_err(|error| {
        PaceError::with_source(ErrorKind::InputOutput, "cannot read {path}", error)
            .attach_context("path", clients_file.display().to_string())
    })?;
    let table = table_text(&clients_file, &text, &[key])?.ok_or_else(|| {
        PaceError::new(
            ErrorKind::Internal,
            "{path} has no table of the client `{key}`, which pace read from it",
        )
        .attach_context("path", clients_file.display().to_string())
        .attach_context("key", key)
        .ask_report()
    })?;
    let replace = |edited: &str| FieldEdit::replace_table(&clients_file, &[key], edited);
    let command = format!("pace client edit {key}");
    let (edited, catalog) = edit_until_checked(
        EditedFile {
            path: &clients_file,
            command: &command,
        },
        &table,
        asker,
        edit,
        |edited| {
            let whole = apply_field_edits(&clients_file, &text, &[replace(edited)])?;
            read_catalog_with(home, &clients_file, &whole)
        },
        report,
    )?;
    write_field_edits(vec![replace(&edited)])?;
    let name = catalog
        .client(&client.key)
        .map_or_else(|| client.name.clone(), |saved| saved.name.clone());
    Ok(format!(
        "Saved the client `{key}` ({name}) in {}.",
        clients_file.display()
    ))
}

/// The error of `pace client edit` without a terminal and without an
/// editor: it names the file to edit by hand.
fn no_terminal_to_edit(clients_file: &Path, key: &ClientKey) -> Box<PaceError> {
    PaceError::new(
        ErrorKind::InvalidInput,
        "`pace client edit` has no terminal and no editor in VISUAL or EDITOR to open the client `{key}`",
    )
    .attach_context("key", key.to_string())
    .attach_hint(format!(
        "edit {} by hand, or set VISUAL or EDITOR",
        clients_file.display()
    ))
    .attach_exit_code(ExitCode::Usage)
}

/// `text` as a client key; a usage error if it is none.
fn client_key(text: &str) -> PaceResult<ClientKey> {
    text.parse().map_err(usage)
}

#[cfg(test)]
mod tests {
    use jiff::{SignedDuration, civil::date};
    use pace_core::{Money, Rate, Rates, RoundingPer};
    use pace_store::catalog::read_catalog;

    use super::*;
    use crate::prompt::Scripted;

    const ACME: &str = "version = 1\n\n# pays late\n[acme]\nname = \"ACME GmbH\"\ncurrency = \"EUR\"\n\n[[acme.rates]]\nhourly = 95.50\nfrom = 2026-11-01\n\n[globex]\nname = \"Globex\"\ncurrency = \"USD\"\n";

    fn client(key: &str, rates: &[(i64, Date)], rounding: Rounding) -> Client {
        let currency = "EUR".parse().unwrap();
        Client {
            key: key.parse().unwrap(),
            name: key.to_uppercase(),
            currency,
            rounding,
            rates: Rates::new(
                rates
                    .iter()
                    .map(|&(minor, from)| Rate {
                        hourly: Money::from_minor(minor, currency),
                        from,
                    })
                    .collect(),
            )
            .unwrap(),
            project_rates: std::collections::BTreeMap::new(),
        }
    }

    fn catalog(clients: Vec<Client>) -> Catalog {
        Catalog::new(Vec::new(), clients, Vec::new()).unwrap()
    }

    #[test]
    fn list_shows_rate_of_today_and_rounding() {
        let fifteen = Rounding::new(
            RoundingPer::Entry,
            SignedDuration::from_mins(15),
            RoundingMode::Up,
        )
        .unwrap();
        let catalog = catalog(vec![
            client(
                "acme",
                &[(9550, date(2026, 1, 1)), (10500, date(2027, 1, 1))],
                fifteen,
            ),
            client("later", &[(8000, date(2026, 12, 1))], Rounding::NONE),
        ]);

        assert_eq!(
            client_lines(&catalog, date(2026, 11, 26)),
            [
                "key    name   currency  rate today   rounding",
                "acme   ACME   EUR       95.50        up to 15m per entry",
                "later  LATER  EUR       no rate yet  none",
            ]
        );
    }

    #[test]
    fn list_without_clients_names_client_add() {
        assert_eq!(
            client_lines(&Catalog::default(), date(2026, 11, 26)),
            ["There are no clients yet; `pace client add <key>` adds one."]
        );
    }

    #[test]
    fn rounding_reads_as_words() {
        let rounding = |mode| Rounding::new(RoundingPer::Day, SignedDuration::from_mins(6), mode);

        assert_eq!(
            rounding_words(rounding(RoundingMode::Down).unwrap()).as_deref(),
            Some("down to 6m per day")
        );
        assert_eq!(
            rounding_words(rounding(RoundingMode::Nearest).unwrap()).as_deref(),
            Some("to the nearest 6m per day")
        );
        assert_eq!(rounding_words(Rounding::NONE), None);
    }

    #[test]
    fn edit_of_unknown_client_names_closest_key() {
        let catalog = catalog(vec![client("acme", &[], Rounding::NONE)]);

        let error = known_client(&catalog, &"amce".parse().unwrap()).unwrap_err();

        assert_eq!(error.to_string(), "pace doesn't know the client `amce`");
        assert_eq!(error.hint(), Some("did you mean `acme`?"));
    }

    /// An editor that turns the text it gets into `saved` in turn.
    fn saving(saved: Vec<String>) -> impl FnMut(&str) -> PaceResult<String> {
        let mut saved: Vec<String> = saved.into_iter().rev().collect();
        move |_| {
            saved.pop().ok_or_else(|| {
                PaceError::new(ErrorKind::Internal, "the test has no more saved texts")
            })
        }
    }

    /// The scratch home with [`ACME`] as its clients file, and the client
    /// `acme` as pace reads it.
    fn home_with_acme() -> (tempfile::TempDir, Client) {
        let home = pace_store::testing::scratch_dir().unwrap();
        std::fs::write(home.path().join(CLIENTS_FILE), ACME).unwrap();
        let catalog = read_catalog(home.path()).unwrap();
        let acme = catalog.client(&"acme".parse().unwrap()).unwrap().clone();
        (home, acme)
    }

    #[test]
    fn edit_opens_table_alone_and_writes_it_in_its_place() {
        let (home, acme) = home_with_acme();
        let opened = std::cell::RefCell::new(String::new());

        let line = edit_client_table(
            home.path(),
            &acme,
            &Scripted::without_terminal(),
            |text| {
                opened.borrow_mut().push_str(text);
                Ok(text.replace("ACME GmbH", "ACME AG"))
            },
            |_| {},
        )
        .unwrap();

        assert_eq!(
            opened.into_inner(),
            "# pays late\n[acme]\nname = \"ACME GmbH\"\ncurrency = \"EUR\"\n\n[[acme.rates]]\nhourly = 95.50\nfrom = 2026-11-01\n"
        );
        assert_eq!(
            std::fs::read_to_string(home.path().join(CLIENTS_FILE)).unwrap(),
            ACME.replace("ACME GmbH", "ACME AG")
        );
        assert!(
            line.starts_with("Saved the client `acme` (ACME AG) in "),
            "{line}"
        );
    }

    #[test]
    fn edit_with_mistake_opens_again_and_writes_fixed_table() {
        let (home, acme) = home_with_acme();
        let table = table_text(&home.path().join(CLIENTS_FILE), ACME, &["acme"])
            .unwrap()
            .unwrap();
        let broken = table.replace("hourly = 95.50", "hourly = 95.555");
        let fixed = table.replace("hourly = 95.50", "hourly = 99");
        let asker = Scripted::confirming([true]);
        let reported = std::cell::RefCell::new(Vec::new());

        _ = edit_client_table(
            home.path(),
            &acme,
            &asker,
            saving(vec![broken, fixed]),
            |error| reported.borrow_mut().push(error.to_string()),
        )
        .unwrap();

        assert_eq!(reported.into_inner(), ["`95.555` is not an amount in EUR"]);
        assert_eq!(
            std::fs::read_to_string(home.path().join(CLIENTS_FILE)).unwrap(),
            ACME.replace("hourly = 95.50", "hourly = 99")
        );
    }

    #[test]
    fn edit_with_other_client_in_text_writes_nothing() {
        let (home, acme) = home_with_acme();
        let asker = Scripted::confirming([false]);
        let reported = std::cell::RefCell::new(Vec::new());

        let error = edit_client_table(
            home.path(),
            &acme,
            &asker,
            saving(vec![
                "[initech]\nname = \"Initech\"\ncurrency = \"EUR\"\n".to_owned(),
            ]),
            |error| reported.borrow_mut().push(error.to_string()),
        )
        .unwrap_err();

        assert_eq!(
            reported.into_inner(),
            ["the text for the table `acme` holds `initech`, outside of the table"]
        );
        assert_eq!(error.hint(), Some("`pace client edit acme` opens it again"));
        assert_eq!(
            std::fs::read_to_string(home.path().join(CLIENTS_FILE)).unwrap(),
            ACME
        );
    }

    #[test]
    fn edit_without_clients_file_cannot_read_it() {
        let home = pace_store::testing::scratch_dir().unwrap();
        let acme = client("acme", &[], Rounding::NONE);

        let error = edit_client_table(
            home.path(),
            &acme,
            &Scripted::without_terminal(),
            saving(Vec::new()),
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InputOutput);
    }

    #[test]
    fn edit_without_terminal_or_editor_names_clients_file() {
        let error = no_terminal_to_edit(Path::new("clients.toml"), &"acme".parse().unwrap());

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.hint(),
            Some("edit clients.toml by hand, or set VISUAL or EDITOR")
        );
    }
}
