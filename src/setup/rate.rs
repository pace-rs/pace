//! The questions of a client's new rate in `pace setup`: the hourly rate,
//! with the current one as the default, and its first day.
//!
//! The new rate is appended, so the rates before it, and the bills that
//! used them, stay.

use std::path::{Path, PathBuf};

use jiff::civil::Date;
use pace_core::{Client, Money, Rate, Rates};
use pace_store::field_edit::{FieldEdit, FieldValue};

use crate::questions::{Answer, Question, QuestionKind, told};
use crate::setup::summary::SetupChange;

/// The flow of a new rate of one client: it asks for the hourly rate, and
/// for its first day unless the rate stays as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRateFlow {
    clients_file: PathBuf,
    client: Client,
    today: Date,
    hourly: Option<Money>,
    kept: bool,
    from: Option<Date>,
}

impl NewRateFlow {
    /// The flow of a new rate of `client` in `clients_file`; it holds
    /// from `today` unless the answer says otherwise.
    #[must_use]
    pub fn new(clients_file: &Path, client: Client, today: Date) -> Self {
        Self {
            clients_file: clients_file.to_path_buf(),
            client,
            today,
            hourly: None,
            kept: false,
            from: None,
        }
    }

    /// The rate that holds today, if any.
    fn current(&self) -> Option<&Rate> {
        self.client.rates.on(self.today)
    }

    /// The client's latest rate, if any: a new rate begins after it.
    fn latest(&self) -> Option<&Rate> {
        self.client.rates.iter().last()
    }

    /// The first day a new rate may begin on, the default of its first
    /// day: today, or the day after the latest rate's first day if that
    /// is today or later.
    fn first_free_day(&self) -> Date {
        self.latest()
            .and_then(|latest| latest.from.tomorrow().ok())
            .map_or(self.today, |after| after.max(self.today))
    }

    /// The next question; `None` once every question has its answer.
    #[must_use]
    pub fn next_question(&self) -> Option<Question> {
        if self.hourly.is_none() && !self.kept {
            return Some(Question {
                id: "rate",
                text: format!(
                    "What is the new hourly rate of `{}` in {}, such as 95.50?",
                    self.client.key,
                    self.client.currency.code()
                ),
                kind: QuestionKind::Text {
                    default: self.current().map(|rate| rate.hourly.to_string()),
                },
                flag: "--rate",
                flag_answer: None,
                default_note: String::new(),
            });
        }
        (!self.kept && self.from.is_none()).then(|| Question {
            id: "rate-from",
            text: "From which day does the new rate hold?".to_owned(),
            kind: QuestionKind::Text {
                default: Some(self.first_free_day().to_string()),
            },
            flag: "--from",
            flag_answer: None,
            default_note: String::new(),
        })
    }

    /// Take the answer to the question [`NewRateFlow::next_question`]
    /// gave.
    ///
    /// # Errors
    ///
    /// If the answer is no amount, or no day after the latest rate's first
    /// day: the message says why.
    pub fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        let Some(question) = self.next_question() else {
            return Err("every question has its answer".to_owned());
        };
        match (question.id, answer) {
            ("rate", Answer::Text(text)) => {
                let hourly = Money::parse_decimal(text.trim(), self.client.currency)
                    .map_err(|error| told(&error))?;
                if self.current().is_some_and(|rate| rate.hourly == hourly) {
                    self.kept = true;
                } else {
                    self.hourly = Some(hourly);
                }
            }
            ("rate-from", Answer::Text(text)) => {
                let text = text.trim();
                let from: Date = text
                    .parse()
                    .map_err(|_| format!("`{text}` is not a day: write it like `2026-11-01`"))?;
                if let Some(latest) = self.latest()
                    && from <= latest.from
                {
                    return Err(format!(
                        "the rate of `{}` from {} is its latest, so a new one begins after it",
                        self.client.key, latest.from
                    ));
                }
                self.from = Some(from);
            }
            (id, answer) => return Err(format!("{answer:?} is no answer to `{id}`")),
        }
        Ok(())
    }

    /// The new rate's change of the clients file and the client with it;
    /// `None` if the rate stays as it is.
    #[must_use]
    pub fn finish(self) -> Option<(SetupChange, Client)> {
        let (hourly, from) = (self.hourly?, self.from?);
        let mut client = self.client;
        let rates = client
            .rates
            .iter()
            .copied()
            .chain([Rate { hourly, from }])
            .collect();
        client.rates = Rates::new(rates).ok()?;
        let key = client.key.to_string();
        let edit = FieldEdit::append_table(
            &self.clients_file,
            &[&key, "rates"],
            vec![
                ("hourly".to_owned(), FieldValue::Decimal(hourly.to_string())),
                ("from".to_owned(), FieldValue::Date(from)),
            ],
        );
        Some((SetupChange::added(edit), client))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jiff::civil::date;
    use pace_core::Rounding;

    use super::*;

    const TODAY: Date = date(2026, 11, 27);

    fn acme(rates: &[(&str, Date)]) -> Client {
        let currency = "EUR".parse().unwrap();
        Client {
            key: "acme".parse().unwrap(),
            name: "ACME".to_owned(),
            currency,
            rounding: Rounding::NONE,
            rates: Rates::new(
                rates
                    .iter()
                    .map(|(hourly, from)| Rate {
                        hourly: Money::parse_decimal(hourly, currency).unwrap(),
                        from: *from,
                    })
                    .collect(),
            )
            .unwrap(),
            project_rates: BTreeMap::new(),
        }
    }

    fn flow(client: Client) -> NewRateFlow {
        NewRateFlow::new(Path::new("clients.toml"), client, TODAY)
    }

    #[test]
    fn current_rate_is_default_and_enter_keeps_it() {
        let mut flow = flow(acme(&[("95.00", date(2026, 1, 1))]));
        let question = flow.next_question().unwrap();

        assert_eq!(
            question.kind,
            QuestionKind::Text {
                default: Some("95.00".to_owned())
            }
        );
        flow.take_answer(Answer::Text("95".to_owned())).unwrap();

        assert_eq!(flow.next_question(), None);
        assert_eq!(flow.finish(), None);
    }

    #[test]
    fn new_rate_is_appended_from_its_day() {
        let mut flow = flow(acme(&[("95.00", date(2026, 1, 1))]));
        flow.take_answer(Answer::Text("105.5".to_owned())).unwrap();
        flow.take_answer(Answer::Text("2026-12-01".to_owned()))
            .unwrap();

        let (change, client) = flow.finish().unwrap();

        assert_eq!(
            change,
            SetupChange::added(FieldEdit::append_table(
                "clients.toml",
                &["acme", "rates"],
                vec![
                    (
                        "hourly".to_owned(),
                        FieldValue::Decimal("105.50".to_owned())
                    ),
                    ("from".to_owned(), FieldValue::Date(date(2026, 12, 1))),
                ],
            ))
        );
        assert_eq!(client.rates.iter().count(), 2);
    }

    #[test]
    fn day_not_after_latest_rate_is_told() {
        let mut flow = flow(acme(&[("95.00", date(2026, 12, 1))]));
        flow.take_answer(Answer::Text("105".to_owned())).unwrap();

        let message = flow
            .take_answer(Answer::Text("2026-12-01".to_owned()))
            .unwrap_err();

        assert_eq!(
            message,
            "the rate of `acme` from 2026-12-01 is its latest, so a new one begins after it"
        );
    }

    #[test]
    fn client_without_rate_has_no_default() {
        let flow = flow(acme(&[]));

        assert_eq!(
            flow.next_question().unwrap().kind,
            QuestionKind::Text { default: None }
        );
    }

    #[test]
    fn amount_that_is_no_decimal_is_told() {
        let mut flow = flow(acme(&[]));

        let message = flow
            .take_answer(Answer::Text("lots".to_owned()))
            .unwrap_err();

        assert!(message.contains("lots"), "{message}");
    }

    #[test]
    fn default_day_follows_latest_rate_that_starts_today() {
        let mut flow = flow(acme(&[("95.00", TODAY)]));
        flow.take_answer(Answer::Text("105".to_owned())).unwrap();
        let question = flow.next_question().unwrap();

        flow.take_answer(question.default_answer().unwrap())
            .unwrap();

        assert_eq!(
            question.kind,
            QuestionKind::Text {
                default: Some("2026-11-28".to_owned())
            }
        );
    }

    #[test]
    fn default_day_is_today_after_past_rate() {
        let mut flow = flow(acme(&[("95.00", date(2026, 1, 1))]));
        flow.take_answer(Answer::Text("105".to_owned())).unwrap();

        assert_eq!(
            flow.next_question().unwrap().kind,
            QuestionKind::Text {
                default: Some("2026-11-27".to_owned())
            }
        );
    }
}
