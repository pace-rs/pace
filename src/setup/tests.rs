//! Tests of the setup machine: each step with answers given in order, the
//! defaults of a first and a second run, the summary and the saved state.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use jiff::civil::date;
use pace_core::testing::SequentialIds;
use pace_core::{Money, Rate, Rates, RoleKind, Rounding};

use crate::questions::QuestionKind;

use super::*;

const TODAY: Date = date(2026, 11, 27);
const CONFIG: &str = "/config/pace.global.toml";
const HOME: &str = "/data/pace";
const DIR: &str = "/work/webshop";
const PROJECT_ID: &str = "0193a7c4-0000-7000-8000-000000000001";

/// The inputs of a first run: the default settings, no roles, no
/// clients, a directory without a manifest.
fn first_run() -> SetupInputs {
    SetupInputs {
        config_file: PathBuf::from(CONFIG),
        time_zone: "system".to_owned(),
        week_start: "monday".to_owned(),
        home: PathBuf::from(HOME),
        catalog: Catalog::default(),
        dir: PathBuf::from(DIR),
        manifest: None,
        today: TODAY,
        ids: Box::new(SequentialIds::starting_at(
            &TODAY.at(9, 0, 0, 0).in_tz("UTC").unwrap(),
            1,
        )),
    }
}

fn acme() -> Client {
    let currency = "EUR".parse().unwrap();
    Client {
        key: "acme".parse().unwrap(),
        name: "ACME".to_owned(),
        currency,
        rounding: Rounding::NONE,
        rates: Rates::new(vec![Rate {
            hourly: Money::parse_decimal("95.00", currency).unwrap(),
            from: date(2026, 1, 1),
        }])
        .unwrap(),
        project_rates: BTreeMap::new(),
    }
}

fn work() -> Role {
    Role {
        id: "0193a7c4-0000-7000-8000-000000000002".parse().unwrap(),
        key: "work".parse().unwrap(),
        kind: RoleKind::Area,
        name: "Work".to_owned(),
        purpose: None,
        weekly_target: None,
        archived: None,
    }
}

fn webshop() -> ProjectRecord {
    ProjectRecord {
        id: PROJECT_ID.parse().unwrap(),
        key: "webshop".parse().unwrap(),
        name: "Webshop".to_owned(),
        client: Some("acme".parse().unwrap()),
        role: Some("work".parse().unwrap()),
        billable: true,
        budget: None,
        starts_on: None,
        ends_on: None,
        done_when: None,
        allow_overlap: false,
        archived: None,
    }
}

/// The inputs of a second run: settings in the global file, the role
/// `work`, the client `acme` and this directory linked to `webshop`.
fn second_run() -> SetupInputs {
    SetupInputs {
        time_zone: "Europe/Berlin".to_owned(),
        week_start: "sunday".to_owned(),
        catalog: Catalog::new(vec![work()], vec![acme()], vec![webshop()]).unwrap(),
        manifest: Some(Manifest {
            path: PathBuf::from(DIR).join(".pace.toml"),
            id: Some(PROJECT_ID.parse().unwrap()),
            key: "webshop".parse().unwrap(),
            name: None,
            default_tags: BTreeSet::new(),
        }),
        ..first_run()
    }
}

/// What a run of setup asked, and its flow.
struct Run {
    flow: SetupFlow,
    asked: Vec<Question>,
    /// The summaries shown before a question.
    summaries: Vec<Vec<String>>,
}

impl Run {
    fn ids(&self) -> Vec<&'static str> {
        self.asked.iter().map(|question| question.id).collect()
    }

    fn question(&self, id: &str) -> &Question {
        self.asked
            .iter()
            .rev()
            .find(|question| question.id == id)
            .unwrap()
    }

    fn finish(self) -> Ended<SetupOutcome> {
        self.flow.finish().unwrap()
    }
}

/// Answer each question of setup over `inputs` with `answers` in order
/// where it names the question, else with the default, as Enter does.
fn run(inputs: SetupInputs, answers: &[(&str, Answer)]) -> Run {
    let mut flow = SetupFlow::new(inputs);
    let mut answers: VecDeque<_> = answers.iter().cloned().collect();
    let mut asked = Vec::new();
    let mut summaries = Vec::new();
    while let Some(question) = flow.next_question() {
        summaries.extend(flow.summary());
        let answer = match answers.front() {
            Some((id, _)) if *id == question.id => answers.pop_front().unwrap().1,
            _ => question
                .default_answer()
                .unwrap_or_else(|| panic!("no answer to `{}`", question.id)),
        };
        flow.take_answer(answer).unwrap();
        asked.push(question);
    }
    assert!(answers.is_empty(), "unasked: {answers:?}");
    Run {
        flow,
        asked,
        summaries,
    }
}

fn text(text: &str) -> Answer {
    Answer::Text(text.to_owned())
}

#[test]
fn first_run_with_all_defaults_asks_each_step_and_changes_nothing() {
    let run = run(first_run(), &[]);

    assert_eq!(
        run.ids(),
        [
            "time-zone",
            "week-start",
            "add-role",
            "clients",
            "directory"
        ]
    );
    assert_eq!(
        run.question("add-role").text,
        "You have no roles yet. Add one?"
    );
    assert_eq!(
        run.question("directory").text,
        "Make /work/webshop a project?"
    );
    let ended = run.finish();
    assert_eq!(ended.made, SetupOutcome::NothingToChange);
    assert_eq!(ended.edits, []);
}

#[test]
fn second_run_shows_current_values_and_with_nothing_changed_ends_without_edits() {
    let run = run(second_run(), &[("directory", Answer::YesNo(true))]);

    assert_eq!(
        run.question("time-zone").kind,
        QuestionKind::Text {
            default: Some("Europe/Berlin".to_owned())
        }
    );
    assert_eq!(
        run.question("week-start").default_answer(),
        Some(Answer::Choice(6))
    );
    assert_eq!(
        run.question("add-role").text,
        "You have the role `work`. Add another?"
    );
    assert_eq!(
        run.question("directory").text,
        "Change the record of the project `webshop` (Webshop)?"
    );
    assert!(run.ids().contains(&"record-billable"));
    let ended = run.finish();
    assert_eq!(ended.made, SetupOutcome::NothingToChange);
    assert_eq!(ended.edits, []);
}

#[test]
fn changed_time_zone_gives_one_edit_and_summary_line() {
    let run = run(second_run(), &[("time-zone", text("europe/vienna"))]);

    assert_eq!(run.question("accept").text, "Write these changes?");
    assert_eq!(
        run.summaries,
        [["pace.global.toml: time-zone Europe/Berlin → Europe/Vienna"]]
    );
    let ended = run.finish();
    assert_eq!(
        ended.edits,
        [FieldEdit::set(
            CONFIG,
            &["time-zone"],
            FieldValue::String("Europe/Vienna".to_owned())
        )]
    );
    assert_eq!(
        ended.made,
        SetupOutcome::Accepted {
            changes: vec!["pace.global.toml: time-zone Europe/Berlin → Europe/Vienna".to_owned()],
            files: vec!["pace.global.toml".to_owned()],
        }
    );
}

#[test]
fn time_zone_that_pace_does_not_know_is_told_and_asked_again() {
    let mut flow = SetupFlow::new(first_run());

    let message = flow.take_answer(text("Mars/Olympus")).unwrap_err();

    assert!(
        message.contains("`Mars/Olympus` is not a time zone"),
        "{message}"
    );
    assert_eq!(flow.next_question().unwrap().id, "time-zone");
}

#[test]
fn changed_week_start_is_change_of_global_file() {
    let ended = run(first_run(), &[("week-start", Answer::Choice(6))]).finish();

    assert_eq!(
        ended.edits,
        [FieldEdit::set(
            CONFIG,
            &["week-start"],
            FieldValue::String("sunday".to_owned())
        )]
    );
}

#[test]
fn declined_summary_writes_nothing() {
    let ended = run(
        first_run(),
        &[
            ("week-start", Answer::Choice(6)),
            ("accept", Answer::YesNo(false)),
        ],
    )
    .finish();

    assert_eq!(ended.edits, []);
    assert_eq!(
        ended.made,
        SetupOutcome::Declined {
            changes: vec!["pace.global.toml: week-start monday → sunday".to_owned()]
        }
    );
}

#[test]
fn new_role_is_asked_as_role_add_asks_it_and_listed_after() {
    let run = run(
        first_run(),
        &[
            ("add-role", Answer::YesNo(true)),
            ("role-key", text("health")),
            ("weekly-target", text("5h")),
        ],
    );

    assert_eq!(
        run.ids()[2..9],
        [
            "add-role",
            "role-key",
            "name",
            "kind",
            "purpose",
            "weekly-target",
            "add-role"
        ]
    );
    assert_eq!(
        run.question("add-role").text,
        "You have the role `health`. Add another?"
    );
    let Ended { edits, made } = run.finish();
    assert!(matches!(made, SetupOutcome::Accepted { files, .. } if files == ["roles.toml"]));
    assert_eq!(
        edits.last(),
        Some(&FieldEdit::set(
            Path::new(HOME).join("roles.toml"),
            &["health", "weekly-target"],
            FieldValue::String("5h".to_owned())
        ))
    );
}

#[test]
fn role_key_that_exists_is_told_and_asked_again() {
    let mut flow = SetupFlow::new(second_run());
    for answer in [
        text("Europe/Berlin"),
        Answer::Choice(6),
        Answer::YesNo(true),
    ] {
        flow.take_answer(answer).unwrap();
    }

    let message = flow.take_answer(text("work")).unwrap_err();

    assert!(
        message.contains("the role `work` exists already"),
        "{message}"
    );
    assert_eq!(flow.next_question().unwrap().id, "role-key");
}

#[test]
fn new_client_is_asked_as_client_add_asks_it() {
    let ended = run(
        first_run(),
        &[
            ("clients", Answer::Choice(1)),
            ("client-key", text("globex")),
            ("currency", text("usd")),
        ],
    )
    .finish();

    let file = Path::new(HOME).join("clients.toml");
    assert_eq!(
        ended.edits,
        [
            FieldEdit::set(
                &file,
                &["globex", "name"],
                FieldValue::String("globex".to_owned())
            ),
            FieldEdit::set(
                &file,
                &["globex", "currency"],
                FieldValue::String("USD".to_owned())
            ),
        ]
    );
}

#[test]
fn new_rate_of_client_is_appended_from_today() {
    let run = run(
        second_run(),
        &[("clients", Answer::Choice(2)), ("rate", text("105"))],
    );

    assert_eq!(
        run.question("clients").kind,
        QuestionKind::Choice {
            items: vec![
                "go on".to_owned(),
                "add a client".to_owned(),
                "give `acme` a new rate".to_owned()
            ],
            default: 0
        }
    );
    let ended = run.finish();
    assert_eq!(
        ended.made,
        SetupOutcome::Accepted {
            changes: vec!["clients.toml: acme.rates + hourly 105.00, from 2026-11-27".to_owned()],
            files: vec!["clients.toml".to_owned()],
        }
    );
}

#[test]
fn same_rate_again_changes_nothing() {
    let ended = run(second_run(), &[("clients", Answer::Choice(2))]).finish();

    assert_eq!(ended.made, SetupOutcome::NothingToChange);
}

#[test]
fn directory_without_manifest_becomes_project_with_questions_of_project_init() {
    let run = run(first_run(), &[("directory", Answer::YesNo(true))]);

    assert_eq!(
        run.ids()[5..],
        ["key", "name", "client", "role", "billable", "accept"]
    );
    let ended = run.finish();
    assert_eq!(
        ended.made,
        SetupOutcome::Accepted {
            changes: vec![
                format!("projects/webshop.toml: id → {}", record_id(&ended.edits)),
                "projects/webshop.toml: key → webshop".to_owned(),
                "projects/webshop.toml: name → webshop".to_owned(),
                "projects/webshop.toml: billable → false".to_owned(),
                format!(".pace.toml: project.id → {}", record_id(&ended.edits)),
                ".pace.toml: project.key → webshop".to_owned(),
            ],
            files: vec!["projects/webshop.toml".to_owned(), ".pace.toml".to_owned()],
        }
    );
}

/// The ID that the first of `edits` sets: the new record's.
fn record_id(edits: &[FieldEdit]) -> String {
    match &edits[0].change {
        FieldChange::Set(FieldValue::String(id)) => id.clone(),
        other => panic!("no ID: {other:?}"),
    }
}

#[test]
fn changed_record_field_is_its_only_edit() {
    let ended = run(
        second_run(),
        &[
            ("directory", Answer::YesNo(true)),
            ("record-billable", Answer::YesNo(false)),
        ],
    )
    .finish();

    assert_eq!(
        ended.edits,
        [FieldEdit::set(
            Path::new(HOME).join("projects/webshop.toml"),
            &["billable"],
            FieldValue::Boolean(false)
        )]
    );
}

#[test]
fn record_client_that_breaks_client_project_rates_is_told_and_asked_again() {
    let mut client = acme();
    client.project_rates = BTreeMap::from([("webshop".parse().unwrap(), client.rates.clone())]);
    let inputs = SetupInputs {
        catalog: Catalog::new(vec![work()], vec![client], vec![webshop()]).unwrap(),
        ..second_run()
    };
    let mut flow = SetupFlow::new(inputs);
    while let Some(question) = flow.next_question() {
        if question.id == "record-client" {
            break;
        }
        let answer = match question.id {
            "directory" => Answer::YesNo(true),
            _ => question.default_answer().unwrap(),
        };
        flow.take_answer(answer).unwrap();
    }

    let message = flow.take_answer(Answer::Choice(0)).unwrap_err();

    assert!(
        message.starts_with("the client `acme` has rates for the project `webshop`"),
        "{message}"
    );
    assert_eq!(flow.next_question().unwrap().id, "record-client");
}

/// The flow of a first run that answered `answers`.
fn answered(inputs: SetupInputs, answers: impl IntoIterator<Item = Answer>) -> SetupFlow {
    let mut flow = SetupFlow::new(inputs);
    for answer in answers {
        flow.take_answer(answer).unwrap();
    }
    flow
}

#[test]
fn saved_setup_serializes_and_replays_to_same_question() {
    let flow = answered(
        first_run(),
        [
            text("UTC"),
            Answer::Choice(0),
            Answer::YesNo(true),
            text("work"),
        ],
    );

    let saved = flow.saved();
    let json = serde_json::to_string(&saved).unwrap();
    let replayed = SetupFlow::replayed(first_run(), &serde_json::from_str(&json).unwrap());

    assert_eq!(saved.step, SetupStep::Roles);
    assert!(json.starts_with(
        r#"{"version":1,"step":"roles","answers":[{"question":"time-zone","answer":{"text":"UTC"},"replaced":"system"}"#
    ));
    assert_eq!(replayed.changed, None);
    assert_eq!(replayed.flow.saved(), saved);
    assert_eq!(replayed.flow.next_question().unwrap().id, "name");
}

#[test]
fn saved_setup_round_trips_through_toml_with_version_1() {
    let saved = answered(
        second_run(),
        [
            text("UTC"),
            Answer::Choice(6),
            Answer::YesNo(false),
            Answer::Choice(2),
        ],
    )
    .saved();

    let toml = toml::to_string(&saved).unwrap();

    assert!(
        toml.starts_with("version = 1\nstep = \"clients\"\n"),
        "{toml}"
    );
    assert_eq!(toml::from_str::<SavedSetup>(&toml).unwrap(), saved);
    assert_eq!(
        saved.answers.last().unwrap().picked.as_deref(),
        Some("give `acme` a new rate")
    );
}

#[test]
fn saved_answer_of_another_question_stops_replay() {
    let saved = SavedSetup {
        version: SAVED_SETUP_VERSION,
        step: SetupStep::Settings,
        answers: vec![SavedAnswer {
            question: "week-start".to_owned(),
            answer: Answer::Choice(6),
            replaced: Some("monday".to_owned()),
            picked: Some("sunday".to_owned()),
        }],
    };

    let replayed = SetupFlow::replayed(first_run(), &saved);

    assert_eq!(replayed.flow.saved().answers, []);
}

#[test]
fn value_changed_since_save_asks_its_question_again_and_shows_change() {
    let saved = answered(first_run(), [text("UTC"), Answer::Choice(0)]).saved();
    let changed = SetupInputs {
        time_zone: "Europe/Vienna".to_owned(),
        ..first_run()
    };

    let replayed = SetupFlow::replayed(changed, &saved);

    assert_eq!(replayed.flow.next_question().unwrap().id, "time-zone");
    assert_eq!(
        replayed.changed.as_deref(),
        Some("Since setup stopped, the value changed: `system` → `Europe/Vienna`.")
    );
}

#[test]
fn record_field_changed_since_save_asks_again_after_answers_before_it() {
    let saved = answered(
        second_run(),
        [
            text("Europe/Berlin"),
            Answer::Choice(6),
            Answer::YesNo(false),
            Answer::Choice(0),
            Answer::YesNo(true),
            text("Shop"),
        ],
    )
    .saved();
    let renamed = ProjectRecord {
        name: "Web shop".to_owned(),
        ..webshop()
    };
    let changed = SetupInputs {
        catalog: Catalog::new(vec![work()], vec![acme()], vec![renamed]).unwrap(),
        ..second_run()
    };

    let replayed = SetupFlow::replayed(changed, &saved);

    assert_eq!(replayed.flow.saved().answers, saved.answers[..5]);
    assert_eq!(replayed.flow.next_question().unwrap().id, "record-name");
    assert_eq!(
        replayed.changed.as_deref(),
        Some("Since setup stopped, the value changed: `Webshop` → `Web shop`.")
    );
}

#[test]
fn picked_item_that_moved_since_save_is_asked_again() {
    let saved = answered(
        second_run(),
        [
            text("Europe/Berlin"),
            Answer::Choice(6),
            Answer::YesNo(false),
            Answer::Choice(2),
        ],
    )
    .saved();
    let aardvark = Client {
        key: "aardvark".parse().unwrap(),
        ..acme()
    };
    let changed = SetupInputs {
        catalog: Catalog::new(vec![work()], vec![aardvark, acme()], vec![webshop()]).unwrap(),
        ..second_run()
    };

    let replayed = SetupFlow::replayed(changed, &saved);

    assert_eq!(replayed.flow.next_question().unwrap().id, "clients");
    assert_eq!(
        replayed.changed.as_deref(),
        Some("Since setup stopped, the items changed: pick give `acme` a new rate again.")
    );
}

#[test]
fn saved_answer_that_no_longer_fits_is_asked_again_with_refusal() {
    let saved = answered(
        first_run(),
        [
            text("UTC"),
            Answer::Choice(0),
            Answer::YesNo(true),
            text("work"),
        ],
    )
    .saved();
    let changed = SetupInputs {
        catalog: Catalog::new(vec![work()], Vec::new(), Vec::new()).unwrap(),
        ..first_run()
    };

    let replayed = SetupFlow::replayed(changed, &saved);

    assert_eq!(replayed.flow.saved().answers, saved.answers[..3]);
    assert_eq!(replayed.flow.next_question().unwrap().id, "role-key");
    assert_eq!(
        replayed.changed.as_deref(),
        Some(
            "Since setup stopped, your answer `work` no longer fits: the role `work` exists \
             already: change it with `pace role edit`."
        )
    );
}

#[test]
fn week_days_are_values_of_setting() {
    for day in WEEK_DAYS {
        assert_eq!(
            setting_value(Key::WeekStart, day).unwrap().as_str(),
            Some(day)
        );
    }
}
