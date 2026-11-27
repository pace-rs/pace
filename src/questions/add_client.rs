//! The questions of `pace client add`: the client's name, currency, hourly
//! rate and its first day, and the rounding of its billable time.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use jiff::{SignedDuration, civil::Date};
use pace_core::{
    Catalog, Client, ClientKey, Currency, Money, Rate, Rates, Rounding, RoundingMode, RoundingPer,
};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::{
    catalog::{CLIENTS_FILE, VERSION_KEY},
    field_edit::{FieldEdit, FieldValue},
};
use pace_time::parse_duration;

use crate::questions::{Answer, Ended, Flow, Optional, Question, QuestionKind, told};

/// The ways a rounding goes, in the order the question offers them; `up`
/// first, as the default.
const MODES: [RoundingMode; 3] = [RoundingMode::Up, RoundingMode::Down, RoundingMode::Nearest];

/// What a rounding rounds, in the order the question offers it; `entry`
/// first, as the default.
const PERS: [RoundingPer; 2] = [RoundingPer::Entry, RoundingPer::Day];

/// What the flags of `pace client add` gave, as typed; what they leave out
/// is asked. Each value passes its question's check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientFlags {
    /// `--name`
    pub name: Option<String>,
    /// `--currency`, such as `EUR`
    pub currency: Option<String>,
    /// `--rate`, the hourly rate, such as `95.50`
    pub rate: Option<String>,
    /// `--from`, the rate's first day, such as `2026-11-01`
    pub from: Option<String>,
    /// `--rounding-step`, such as `15m`
    pub rounding_step: Option<String>,
    /// `--rounding-mode`
    pub rounding_mode: Option<RoundingMode>,
    /// `--rounding-per`
    pub rounding_per: Option<RoundingPer>,
}

/// An hourly rate as the user typed it, and the amount it is.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TypedRate {
    /// The decimal the clients file gets: `95.50` stays `95.50`.
    text: String,
    hourly: Money,
}

/// The flow of a new client: it asks for what [`ClientFlags`] leave open
/// and ends with the client's table in the clients file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddClientFlow {
    clients_file: PathBuf,
    key: ClientKey,
    today: Date,
    flags: ClientFlags,
    name: Option<String>,
    currency: Option<Currency>,
    rate: Optional<TypedRate>,
    from: Option<Date>,
    step: Optional<SignedDuration>,
    mode: Option<RoundingMode>,
    per: Option<RoundingPer>,
}

impl AddClientFlow {
    /// The flow of the client `key`, written to `clients_file`; a rate
    /// holds from `today` unless `--from` says otherwise, and `flags`
    /// answer their questions.
    ///
    /// # Errors
    ///
    /// If `key` is `version`, which the clients file keeps for its format
    /// (exit code 2), or if `catalog` has a client `key` already.
    pub fn new(
        catalog: &Catalog,
        clients_file: &Path,
        key: ClientKey,
        today: Date,
        flags: ClientFlags,
    ) -> PaceResult<Self> {
        if key.to_string() == VERSION_KEY {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "a client can't have the key `{key}`, which {file} keeps for its format",
            )
            .attach_context("key", VERSION_KEY)
            .attach_context("file", CLIENTS_FILE)
            .attach_hint("give the client another key")
            .attach_exit_code(ExitCode::Usage));
        }
        if catalog.client(&key).is_some() {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "the client `{key}` exists already",
            )
            .attach_context("key", key.to_string())
            .attach_hint(format!("change it with `pace client edit {key}`")));
        }
        Ok(Self {
            clients_file: clients_file.to_path_buf(),
            key,
            today,
            flags,
            name: None,
            currency: None,
            rate: Optional::Unasked,
            from: None,
            step: Optional::Unasked,
            mode: None,
            per: None,
        })
    }

    /// The questions of the rate and its first day; `None` once answered.
    fn rate_question(&self) -> Option<Question> {
        if self.rate == Optional::Unasked {
            return Some(Question {
                id: "rate",
                text: "What is the hourly rate, such as 95.50?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(String::new()),
                },
                flag: "--rate",
                flag_answer: self.flags.rate.clone().map(Answer::Text),
                default_note: "no rate".to_owned(),
            });
        }
        let has_rate = matches!(self.rate, Optional::Answered(Some(_)));
        (has_rate && self.from.is_none()).then(|| Question {
            id: "from",
            text: "From which day does the rate hold?".to_owned(),
            kind: QuestionKind::Text {
                default: Some(self.today.to_string()),
            },
            flag: "--from",
            flag_answer: self.flags.from.clone().map(Answer::Text),
            default_note: format!("the rate from today ({})", self.today),
        })
    }

    /// The questions of the rounding; `None` once answered.
    fn rounding_question(&self) -> Option<Question> {
        if self.step == Optional::Unasked {
            return Some(Question {
                id: "rounding-step",
                text: "To which step does billable time round, such as 15m?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(String::new()),
                },
                flag: "--rounding-step",
                flag_answer: self.flags.rounding_step.clone().map(Answer::Text),
                default_note: "no rounding".to_owned(),
            });
        }
        if !matches!(self.step, Optional::Answered(Some(_))) {
            return None;
        }
        if self.mode.is_none() {
            return Some(Question {
                id: "rounding-mode",
                text: "Which way does it round?".to_owned(),
                kind: QuestionKind::Choice {
                    items: MODES.iter().map(|mode| mode_item(*mode)).collect(),
                    default: 0,
                },
                flag: "--rounding-mode",
                flag_answer: self.flags.rounding_mode.map(|mode| {
                    Answer::Choice(MODES.iter().position(|known| *known == mode).unwrap_or(0))
                }),
                default_note: format!("rounding `{}`", RoundingMode::Up.as_str()),
            });
        }
        self.per.is_none().then(|| Question {
            id: "rounding-per",
            text: "What does it round?".to_owned(),
            kind: QuestionKind::Choice {
                items: PERS.iter().map(|per| per_item(*per)).collect(),
                default: 0,
            },
            flag: "--rounding-per",
            flag_answer: self.flags.rounding_per.map(|per| {
                Answer::Choice(PERS.iter().position(|known| *known == per).unwrap_or(0))
            }),
            default_note: format!("rounding per {}", RoundingPer::Entry.as_str()),
        })
    }

    /// Take `text` as the hourly rate in the client's currency; blank for
    /// none.
    fn take_rate(&mut self, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            self.rate = Optional::Answered(None);
            return Ok(());
        }
        let currency = self.currency.ok_or("the currency comes before the rate")?;
        let hourly = Money::parse_decimal(text, currency).map_err(|error| told(&error))?;
        self.rate = Optional::Answered(Some(TypedRate {
            text: toml_decimal(text),
            hourly,
        }));
        Ok(())
    }

    /// The rounding the answers decided; [`Rounding::NONE`] without a step.
    fn rounding(&self) -> Rounding {
        let Optional::Answered(Some(step)) = self.step else {
            return Rounding::NONE;
        };
        Rounding::new(
            self.per.unwrap_or(RoundingPer::Entry),
            step,
            self.mode.unwrap_or(RoundingMode::Up),
        )
        .unwrap_or(Rounding::NONE)
    }
}

impl Flow for AddClientFlow {
    type Made = Client;

    fn next_question(&self) -> Option<Question> {
        if self.name.is_none() {
            return Some(Question {
                id: "name",
                text: "What is the client's name?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(self.key.to_string()),
                },
                flag: "--name",
                flag_answer: self.flags.name.clone().map(Answer::Text),
                default_note: format!("name `{}` from the key", self.key),
            });
        }
        if self.currency.is_none() {
            return Some(Question {
                id: "currency",
                text: "Which currency does the client pay in, such as EUR?".to_owned(),
                kind: QuestionKind::Text { default: None },
                flag: "--currency",
                flag_answer: self.flags.currency.clone().map(Answer::Text),
                default_note: String::new(),
            });
        }
        self.rate_question().or_else(|| self.rounding_question())
    }

    fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        let Some(question) = self.next_question() else {
            return Err("every question has its answer".to_owned());
        };
        match (question.id, answer) {
            ("name", Answer::Text(name)) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err("a client needs a name".to_owned());
                }
                self.name = Some(name.to_owned());
            }
            ("currency", Answer::Text(code)) => {
                let code = code.trim().to_ascii_uppercase();
                self.currency = Some(code.parse::<Currency>().map_err(|error| told(&error))?);
            }
            ("rate", Answer::Text(rate)) => self.take_rate(&rate)?,
            ("from", Answer::Text(from)) => {
                let from = from.trim();
                self.from =
                    Some(from.parse().map_err(|_| {
                        format!("`{from}` is not a day: write it like `2026-11-01`")
                    })?);
            }
            ("rounding-step", Answer::Text(step)) => {
                let step = step.trim();
                self.step = Optional::Answered(if step.is_empty() {
                    None
                } else {
                    Some(parse_duration(step).map_err(|error| told(&error))?)
                });
            }
            ("rounding-mode", Answer::Choice(index)) => {
                self.mode = Some(*MODES.get(index).ok_or("pick one of the ways")?);
            }
            ("rounding-per", Answer::Choice(index)) => {
                self.per = Some(*PERS.get(index).ok_or("pick what it rounds")?);
            }
            (id, answer) => return Err(format!("{answer:?} is no answer to `{id}`")),
        }
        Ok(())
    }

    fn finish(self) -> PaceResult<Ended<Client>> {
        let rounding = self.rounding();
        let typed = self.rate.clone().into_value();
        let from = self.from.unwrap_or(self.today);
        let rates = typed
            .as_ref()
            .map(|typed| Rate {
                hourly: typed.hourly,
                from,
            })
            .into_iter()
            .collect();
        let currency = self.currency.ok_or_else(|| {
            PaceError::new(
                ErrorKind::Internal,
                "the flow of the client `{key}` ended without its currency",
            )
            .attach_context("key", self.key.to_string())
            .ask_report()
        })?;
        let client = Client {
            key: self.key,
            name: self.name.unwrap_or_default(),
            currency,
            rounding,
            rates: Rates::new(rates).unwrap_or_default(),
            project_rates: BTreeMap::new(),
        };
        let typed_rate = typed.map(|typed| (typed.text, from));
        Ok(Ended {
            edits: client_edits(&self.clients_file, &client, typed_rate),
            made: client,
        })
    }
}

/// The decimal `text`, which [`Money::parse_decimal`] took, as TOML writes
/// it: without leading zeros, which TOML refuses, so `095.50` is `95.50`.
fn toml_decimal(text: &str) -> String {
    let trimmed = text.trim_start_matches('0');
    if trimmed.is_empty() || trimmed.starts_with('.') {
        format!("0{trimmed}")
    } else {
        trimmed.to_owned()
    }
}

/// How the question of the rounding's way shows `mode`.
fn mode_item(mode: RoundingMode) -> String {
    let meaning = match mode {
        RoundingMode::Up => "to the next step",
        RoundingMode::Down => "to the step before",
        RoundingMode::Nearest => "to the nearest step; half a step rounds up",
    };
    format!("{}: {meaning}", mode.as_str())
}

/// How the question of what the rounding rounds shows `per`.
fn per_item(per: RoundingPer) -> String {
    let meaning = match per {
        RoundingPer::Entry => "each entry by itself",
        RoundingPer::Day => "the time of a day and project together",
    };
    format!("{}: {meaning}", per.as_str())
}

/// The field edits that write `client` as its table in `clients_file`, and
/// its rate, if any, with the amount as typed and its first day.
fn client_edits(
    clients_file: &Path,
    client: &Client,
    rate: Option<(String, Date)>,
) -> Vec<FieldEdit> {
    let key = client.key.to_string();
    let field = |name: &str, value: FieldValue| FieldEdit::set(clients_file, &[&key, name], value);
    let mut edits = vec![
        field("name", FieldValue::String(client.name.clone())),
        field(
            "currency",
            FieldValue::String(client.currency.code().to_owned()),
        ),
    ];
    let rounding = &client.rounding;
    if let (Some(per), Some(step), Some(mode)) = (rounding.per(), rounding.step(), rounding.mode())
    {
        let word = |word: &str| FieldValue::String(word.to_owned());
        edits.push(field(
            "rounding",
            FieldValue::InlineTable(vec![
                ("per".to_owned(), word(per.as_str())),
                ("step".to_owned(), FieldValue::String(format!("{step:#}"))),
                ("mode".to_owned(), word(mode.as_str())),
            ]),
        ));
    }
    if let Some((hourly, from)) = rate {
        edits.push(FieldEdit::append_table(
            clients_file,
            &[&key, "rates"],
            vec![
                ("hourly".to_owned(), FieldValue::Decimal(hourly)),
                ("from".to_owned(), FieldValue::Date(from)),
            ],
        ));
    }
    edits
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use pace_error::ExitCode;

    use super::*;
    use crate::prompt::Scripted;
    use crate::questions::ask_flow;

    const TODAY: Date = date(2026, 11, 26);

    fn flow(flags: ClientFlags) -> AddClientFlow {
        AddClientFlow::new(
            &Catalog::default(),
            Path::new("clients.toml"),
            "acme".parse().unwrap(),
            TODAY,
            flags,
        )
        .unwrap()
    }

    fn euro() -> ClientFlags {
        ClientFlags {
            currency: Some("EUR".to_owned()),
            ..ClientFlags::default()
        }
    }

    fn set(key: &str, value: FieldValue) -> FieldEdit {
        FieldEdit::set("clients.toml", &["acme", key], value)
    }

    fn text(value: &str) -> FieldValue {
        FieldValue::String(value.to_owned())
    }

    fn rate(hourly: &str, from: Date) -> FieldEdit {
        FieldEdit::append_table(
            "clients.toml",
            &["acme", "rates"],
            vec![
                ("hourly".to_owned(), FieldValue::Decimal(hourly.to_owned())),
                ("from".to_owned(), FieldValue::Date(from)),
            ],
        )
    }

    #[test]
    fn every_flag_given_asks_nothing() {
        let flags = ClientFlags {
            name: Some("ACME GmbH".to_owned()),
            currency: Some("EUR".to_owned()),
            rate: Some("95.50".to_owned()),
            from: Some("2026-11-01".to_owned()),
            rounding_step: Some("15m".to_owned()),
            rounding_mode: Some(RoundingMode::Nearest),
            rounding_per: Some(RoundingPer::Day),
        };

        let result = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap();

        assert_eq!(
            result.edits,
            [
                set("name", text("ACME GmbH")),
                set("currency", text("EUR")),
                set(
                    "rounding",
                    FieldValue::InlineTable(vec![
                        ("per".to_owned(), text("day")),
                        ("step".to_owned(), text("15m")),
                        ("mode".to_owned(), text("nearest")),
                    ])
                ),
                rate("95.50", date(2026, 11, 1)),
            ]
        );
        assert_eq!(result.defaults, Vec::<String>::new());
    }

    #[test]
    fn no_terminal_takes_defaults_and_names_them() {
        let result = ask_flow(flow(euro()), &Scripted::without_terminal()).unwrap();

        assert_eq!(
            result.edits,
            [set("name", text("acme")), set("currency", text("EUR"))]
        );
        assert_eq!(
            result.defaults,
            ["name `acme` from the key", "no rate", "no rounding"]
        );
        assert_eq!(result.made.rounding, Rounding::NONE);
    }

    #[test]
    fn rate_without_from_holds_from_today() {
        let flags = ClientFlags {
            rate: Some("95".to_owned()),
            ..euro()
        };

        let result = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap();

        assert_eq!(result.edits.last(), Some(&rate("95", TODAY)));
        assert_eq!(
            result.defaults,
            [
                "name `acme` from the key",
                "the rate from today (2026-11-26)",
                "no rounding"
            ]
        );
    }

    #[test]
    fn rounding_step_alone_rounds_up_per_entry() {
        let flags = ClientFlags {
            rounding_step: Some("6m".to_owned()),
            ..euro()
        };

        let result = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap();

        let rounding = result.made.rounding;
        assert_eq!(rounding.mode(), Some(RoundingMode::Up));
        assert_eq!(rounding.per(), Some(RoundingPer::Entry));
        assert_eq!(rounding.step(), Some(SignedDuration::from_mins(6)));
        assert_eq!(
            result.defaults,
            [
                "name `acme` from the key",
                "no rate",
                "rounding `up`",
                "rounding per entry"
            ]
        );
    }

    #[test]
    fn no_terminal_and_no_currency_is_usage_error_naming_currency() {
        let error =
            ask_flow(flow(ClientFlags::default()), &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(error.hint(), Some("answer it with `--currency`"));
    }

    #[test]
    fn terminal_asks_what_flags_leave_open() {
        let asker =
            Scripted::typing(["ACME GmbH", "eur", "105", "2027-01-01", "15m"]).and_choosing([2, 1]);

        let result = ask_flow(flow(ClientFlags::default()), &asker).unwrap();

        assert_eq!(result.made.currency.code(), "EUR");
        assert_eq!(result.edits.last(), Some(&rate("105", date(2027, 1, 1))));
        assert_eq!(result.made.rounding.mode(), Some(RoundingMode::Nearest));
        assert_eq!(result.made.rounding.per(), Some(RoundingPer::Day));
        assert_eq!(
            asker.questions(),
            [
                "What is the client's name?",
                "Which currency does the client pay in, such as EUR?",
                "What is the hourly rate, such as 95.50?",
                "From which day does the rate hold?",
                "To which step does billable time round, such as 15m?",
                "Which way does it round?",
                "What does it round?",
            ]
        );
        assert_eq!(
            asker.offered(),
            [
                vec![
                    "up: to the next step",
                    "down: to the step before",
                    "nearest: to the nearest step; half a step rounds up"
                ],
                vec![
                    "entry: each entry by itself",
                    "day: the time of a day and project together"
                ]
            ]
        );
    }

    #[test]
    fn answers_that_do_not_fit_are_told_and_asked_again() {
        let asker = Scripted::typing([
            "",
            "EURO",
            "EUR",
            "95.555",
            "95.50",
            "first of may",
            "",
            "0m",
            "",
        ]);

        let result = ask_flow(flow(ClientFlags::default()), &asker).unwrap();

        assert_eq!(result.edits.last(), Some(&rate("95.50", TODAY)));
        assert_eq!(result.made.rounding, Rounding::NONE);
        assert_eq!(
            asker.told(),
            [
                "`EURO` is not an ISO 4217 currency code: did you mean `EUR`?",
                "`95.555` is not an amount in EUR: write an amount in EUR with at most 2 decimals, like `95.50`",
                "`first of may` is not a day: write it like `2026-11-01`",
                "the duration `0m` is not positive: give a duration longer than zero, like `15m`",
            ]
        );
    }

    #[test]
    fn rate_flag_with_too_many_decimals_is_usage_error_naming_what_fits() {
        let flags = ClientFlags {
            rate: Some("95.555".to_owned()),
            ..euro()
        };

        let error = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "`--rate \"95.555\"` doesn't fit: `95.555` is not an amount in EUR: \
             write an amount in EUR with at most 2 decimals, like `95.50`"
        );
    }

    #[test]
    fn blank_name_asks_again() {
        let asker = Scripted::typing(["  ", "ACME", "", ""]);

        let result = ask_flow(flow(euro()), &asker).unwrap();

        assert_eq!(result.made.name, "ACME");
        assert_eq!(asker.told(), ["a client needs a name"]);
    }

    #[test]
    fn rate_with_leading_zero_is_written_as_toml_decimal() {
        assert_eq!(toml_decimal("095.50"), "95.50");
        assert_eq!(toml_decimal("0.5"), "0.5");
        assert_eq!(toml_decimal("000"), "0");
        assert_eq!(toml_decimal("95"), "95");
    }

    #[test]
    fn key_version_is_refused() {
        let error = AddClientFlow::new(
            &Catalog::default(),
            Path::new("clients.toml"),
            "version".parse().unwrap(),
            TODAY,
            euro(),
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "a client can't have the key `version`, which clients.toml keeps for its format"
        );
    }

    #[test]
    fn existing_key_names_client_edit() {
        let acme = ask_flow(flow(euro()), &Scripted::without_terminal())
            .unwrap()
            .made;
        let catalog = Catalog::new(Vec::new(), vec![acme], Vec::new()).unwrap();

        let error = AddClientFlow::new(
            &catalog,
            Path::new("clients.toml"),
            "acme".parse().unwrap(),
            TODAY,
            euro(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "the client `acme` exists already");
        assert_eq!(error.hint(), Some("change it with `pace client edit acme`"));
        assert_eq!(error.exit_code(), ExitCode::Error);
    }
}
