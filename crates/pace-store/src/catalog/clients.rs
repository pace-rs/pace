//! The clients file, `clients.toml`: a table per client key with its name,
//! currency, rounding, dated rates and the rates of its projects
//! (restart.md 3.7).

use std::collections::BTreeMap;

use pace_core::{Client, ClientKey, Currency, Money, ProjectKey, Rate, Rates, Rounding};
use pace_error::PaceResult;

use crate::{
    catalog::{
        Field, SourceMentions,
        fields::{Fields, Owner, duration, invalid_value, parsed, required, text},
    },
    toml_file::{TomlFile, TomlKey, TomlValue},
};

/// The keys of a client's table.
const CLIENT_KEYS: [&str; 5] = ["name", "currency", "rounding", "rates", "projects"];

/// The keys of a client's `rounding`.
const ROUNDING_KEYS: [&str; 3] = ["per", "step", "mode"];

/// The keys of a rate.
const RATE_KEYS: [&str; 2] = ["hourly", "from"];

/// What a client's `rates` take.
const HOURLY: &str = "an amount such as 95.50";

/// The clients of the clients file.
///
/// # Errors
///
/// If it is no valid TOML, has another `version`, or a client that
/// [`read_client`] refuses.
pub(super) fn read_clients(
    file: &TomlFile<'_>,
    mentions: &mut SourceMentions<'_>,
) -> PaceResult<Vec<Client>> {
    let document = file.parse()?;
    let table = document.table();
    file.check_version(table)?;
    table
        .entries()
        .into_iter()
        .filter(|(key, _)| key.name() != "version")
        .map(|(key, value)| read_client(file, key, value, mentions))
        .collect()
}

/// The client under the table `key`.
///
/// # Errors
///
/// If its key is no client key, it is no table, holds an unknown key, lacks
/// its name or currency, or a value doesn't fit.
fn read_client(
    file: &TomlFile<'_>,
    key: TomlKey<'_>,
    value: TomlValue<'_>,
    mentions: &mut SourceMentions<'_>,
) -> PaceResult<Client> {
    let client_key: ClientKey = key
        .name()
        .parse()
        .map_err(|error| invalid_value(file, error, key.span()))?;
    let fields = Fields::of(file, file.table(key.name(), value)?, &CLIENT_KEYS)?;
    let owner = Owner {
        noun: "client",
        key: key.name(),
        span: key.span(),
    };
    let name = required(file, fields.get("name"), &owner, "name")?;
    let currency = required(file, fields.get("currency"), &owner, "currency")?;
    let currency: Currency = parsed(
        file,
        "currency",
        currency,
        "an ISO 4217 code such as \"EUR\"",
    )?;
    Ok(Client {
        key: client_key,
        name: text(file, "name", name, "the client's name")?,
        currency,
        rounding: fields
            .read("rounding", |value| read_rounding(file, value, &owner))?
            .unwrap_or(Rounding::NONE),
        rates: fields
            .read("rates", |value| read_rates(file, value, currency, &owner))?
            .unwrap_or_default(),
        project_rates: fields
            .read("projects", |value| {
                read_project_rates(file, value, currency, &owner, mentions)
            })?
            .unwrap_or_default(),
    })
}

/// A client's `rounding`: `{ per = "entry", step = "15m", mode = "up" }`.
///
/// # Errors
///
/// If it is no table, holds an unknown key, lacks one of its three keys, or
/// a value doesn't fit.
fn read_rounding(
    file: &TomlFile<'_>,
    value: TomlValue<'_>,
    owner: &Owner<'_>,
) -> PaceResult<Rounding> {
    let fields = Fields::of(file, file.table("rounding", value)?, &ROUNDING_KEYS)?;
    let owner = Owner {
        span: value.span(),
        ..owner.clone()
    };
    let per = required(file, fields.get("per"), &owner, "rounding.per")?;
    let step = required(file, fields.get("step"), &owner, "rounding.step")?;
    let mode = required(file, fields.get("mode"), &owner, "rounding.mode")?;
    Rounding::new(
        parsed(file, "per", per, "what the rounding rounds")?,
        duration(file, "step", step)?,
        parsed(file, "mode", mode, "which way the rounding goes")?,
    )
    .map_err(|error| invalid_value(file, error, step.span()))
}

/// A list of rates, `[[acme.rates]]`, each in the client's `currency`.
///
/// # Errors
///
/// If it is no list of tables, a rate holds an unknown key, lacks `hourly`
/// or `from`, or a value doesn't fit; or if two rates begin on one day.
fn read_rates(
    file: &TomlFile<'_>,
    value: TomlValue<'_>,
    currency: Currency,
    owner: &Owner<'_>,
) -> PaceResult<Rates> {
    let mut rates = Vec::new();
    for (table, span) in file.tables("rates", value)? {
        let fields = Fields::of(file, table, &RATE_KEYS)?;
        let owner = Owner {
            noun: "rate of the client",
            span,
            ..owner.clone()
        };
        let hourly = required(file, fields.get("hourly"), &owner, "hourly")?;
        let from = required(file, fields.get("from"), &owner, "from")?;
        let amount = file.decimal_text("hourly", hourly, HOURLY)?;
        rates.push(Rate {
            hourly: Money::parse_decimal(amount, currency)
                .map_err(|error| invalid_value(file, error, hourly.span()))?,
            from: file.date("from", from)?,
        });
    }
    Rates::new(rates).map_err(|error| invalid_value(file, error, value.span()))
}

/// A client's `projects`: the rates of some of its projects, by project
/// key, such as `[[acme.projects.acme-webshop.rates]]`.
///
/// # Errors
///
/// If it is no table, a key is no project key, a project's table holds
/// another key than `rates`, or a rate doesn't fit.
fn read_project_rates(
    file: &TomlFile<'_>,
    value: TomlValue<'_>,
    currency: Currency,
    owner: &Owner<'_>,
    mentions: &mut SourceMentions<'_>,
) -> PaceResult<BTreeMap<ProjectKey, Rates>> {
    let mut project_rates = BTreeMap::new();
    for (key, value) in file.table("projects", value)?.entries() {
        let project: ProjectKey = key
            .name()
            .parse()
            .map_err(|error| invalid_value(file, error, key.span()))?;
        mentions.record(
            "client",
            owner.key,
            Field::ProjectRates(key.name().to_owned()),
            key.span(),
        );
        let fields = Fields::of(file, file.table(key.name(), value)?, &["rates"])?;
        let rates = fields
            .read("rates", |value| read_rates(file, value, currency, owner))?
            .unwrap_or_default();
        _ = project_rates.insert(project, rates);
    }
    Ok(project_rates)
}
