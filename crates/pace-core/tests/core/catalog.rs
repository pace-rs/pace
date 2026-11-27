//! The catalog: roles, clients and project records, their keys and rates.

use std::collections::BTreeMap;

use jiff::{SignedDuration, civil::Date};
use pace_core::{
    ArchiveReason, Catalog, Client, ClientKey, Currency, Money, ProjectId, ProjectRecord,
    ProjectRef, Rate, Rates, Role, RoleId, RoleKey, RoleKind, Rounding, RoundingMode, RoundingPer,
};
use rstest::rstest;

use crate::TestResult;

#[rstest]
#[case("work")]
#[case("area-health")]
fn role_key_is_lowercase_words_with_dashes(#[case] key: &str) -> TestResult {
    assert_eq!(key.parse::<RoleKey>()?.as_str(), key);
    Ok(())
}

#[rstest]
#[case("acme")]
#[case("acme-gmbh-2")]
fn client_key_is_lowercase_words_with_dashes(#[case] key: &str) -> TestResult {
    assert_eq!(key.parse::<ClientKey>()?.to_string(), key);
    Ok(())
}

#[test]
fn role_key_with_uppercase_is_refused_and_named() {
    let error = "Work".parse::<RoleKey>().err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("`Work` is not a role key")
    );
}

#[test]
fn client_key_with_space_is_refused_with_example() {
    let hint = "acme gmbh"
        .parse::<ClientKey>()
        .err()
        .and_then(|error| error.hint().map(ToOwned::to_owned));

    assert_eq!(
        hint.as_deref(),
        Some("a key has lowercase letters, digits and single dashes, like `acme`")
    );
}

#[test]
fn role_id_reads_uuid_v7() -> TestResult {
    let text = "0193a8d0-000d-7000-8000-000000000001";

    assert_eq!(text.parse::<RoleId>()?.to_string(), text);
    Ok(())
}

const fn mins(minutes: i64) -> SignedDuration {
    SignedDuration::from_mins(minutes)
}

fn quarter_hours(mode: RoundingMode) -> Result<Rounding, Box<dyn std::error::Error>> {
    Ok(Rounding::new(RoundingPer::Entry, mins(15), mode)?)
}

#[rstest]
#[case::up(RoundingMode::Up, 46, 60)]
#[case::up_on_step(RoundingMode::Up, 45, 45)]
#[case::down(RoundingMode::Down, 59, 45)]
#[case::nearest_below_half(RoundingMode::Nearest, 52, 45)]
#[case::nearest_below_next_half(RoundingMode::Nearest, 67, 60)]
#[case::nearest_above_half(RoundingMode::Nearest, 53, 60)]
fn rounding_to_quarter_hours(
    #[case] mode: RoundingMode,
    #[case] worked: i64,
    #[case] rounded: i64,
) -> TestResult {
    assert_eq!(quarter_hours(mode)?.round(mins(worked)), mins(rounded));
    Ok(())
}

#[test]
fn half_step_rounds_up_to_nearest() -> TestResult {
    let worked = mins(7) + SignedDuration::from_secs(30);

    assert_eq!(
        quarter_hours(RoundingMode::Nearest)?.round(worked),
        mins(15)
    );
    Ok(())
}

#[test]
fn no_rounding_is_default_and_keeps_duration() {
    let worked = SignedDuration::from_secs(1234);

    assert_eq!(Rounding::default(), Rounding::NONE);
    assert_eq!(Rounding::NONE.round(worked), worked);
    assert_eq!(Rounding::NONE.per(), None);
}

#[test]
fn rounding_keeps_its_rule() -> TestResult {
    let rounding = Rounding::new(RoundingPer::Day, mins(6), RoundingMode::Down)?;

    assert_eq!(
        (rounding.per(), rounding.step(), rounding.mode()),
        (
            Some(RoundingPer::Day),
            Some(mins(6)),
            Some(RoundingMode::Down)
        )
    );
    Ok(())
}

#[rstest]
#[case::zero(SignedDuration::ZERO)]
#[case::negative(mins(-15))]
fn rounding_step_must_be_positive(#[case] step: SignedDuration) {
    let error = Rounding::new(RoundingPer::Entry, step, RoundingMode::Up).err();

    assert!(
        error.is_some_and(|error| error.is_code("pace::core::rounding-step")),
        "a step of {step} rounds nothing"
    );
}

#[rstest]
#[case("area", RoleKind::Area)]
#[case("area-project", RoleKind::AreaProject)]
#[case("meta", RoleKind::Meta)]
fn role_kind_is_written_as_word(#[case] word: &str, #[case] kind: RoleKind) -> TestResult {
    assert_eq!(word.parse::<RoleKind>()?, kind);
    assert_eq!(kind.to_string(), word);
    Ok(())
}

#[rstest]
#[case("completed", ArchiveReason::Completed)]
#[case("abandoned", ArchiveReason::Abandoned)]
#[case("ended", ArchiveReason::Ended)]
#[case("inactive", ArchiveReason::Inactive)]
fn archive_reason_is_written_as_word(
    #[case] word: &str,
    #[case] reason: ArchiveReason,
) -> TestResult {
    assert_eq!(word.parse::<ArchiveReason>()?, reason);
    assert_eq!(reason.to_string(), word);
    Ok(())
}

#[rstest]
#[case("entry", RoundingPer::Entry)]
#[case("day", RoundingPer::Day)]
fn rounding_per_is_written_as_word(#[case] word: &str, #[case] per: RoundingPer) -> TestResult {
    assert_eq!(word.parse::<RoundingPer>()?, per);
    Ok(())
}

#[rstest]
#[case("up", RoundingMode::Up)]
#[case("down", RoundingMode::Down)]
#[case("nearest", RoundingMode::Nearest)]
fn rounding_mode_is_written_as_word(#[case] word: &str, #[case] mode: RoundingMode) -> TestResult {
    assert_eq!(word.parse::<RoundingMode>()?, mode);
    Ok(())
}

#[test]
fn unknown_role_kind_names_choices() {
    let error = "project".parse::<RoleKind>().err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("`project` is not a role kind")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("a role kind is one of `area`, `area-project`, `meta`")
    );
}

type Built<T> = Result<T, Box<dyn std::error::Error>>;

const WORK_ID: &str = "0193a8d0-000d-7000-8000-000000000001";
const WEBSHOP_ID: &str = "0193a8d0-000d-7000-8000-000000000002";
const GARDEN_ID: &str = "0193a8d0-000d-7000-8000-000000000003";

fn day(text: &str) -> Built<Date> {
    Ok(text.parse()?)
}

fn rate(hourly: &str, from: &str) -> Built<Rate> {
    Ok(Rate {
        hourly: Money::parse_decimal(hourly, "EUR".parse()?)?,
        from: day(from)?,
    })
}

fn work() -> Built<Role> {
    Ok(Role {
        id: WORK_ID.parse()?,
        key: "work".parse()?,
        kind: RoleKind::Area,
        name: "Work".to_owned(),
        purpose: None,
        weekly_target: Some(SignedDuration::from_hours(30)),
        archived: None,
    })
}

/// ACME: 95 from 2025-01-01, 105 from 2026-07-01; the webshop 120 from
/// 2026-09-01.
fn acme() -> Built<Client> {
    Ok(Client {
        key: "acme".parse()?,
        name: "ACME GmbH".to_owned(),
        currency: "EUR".parse()?,
        rounding: Rounding::NONE,
        rates: Rates::new(vec![rate("105", "2026-07-01")?, rate("95", "2025-01-01")?])?,
        project_rates: BTreeMap::from([(
            "acme-webshop".parse()?,
            Rates::new(vec![rate("120", "2026-09-01")?])?,
        )]),
    })
}

fn project(id: &str, key: &str) -> Built<ProjectRecord> {
    Ok(ProjectRecord {
        id: id.parse()?,
        key: key.parse()?,
        name: key.to_owned(),
        client: None,
        role: None,
        billable: false,
        budget: None,
        starts_on: None,
        ends_on: None,
        done_when: None,
        allow_overlap: false,
        archived: None,
    })
}

fn webshop() -> Built<ProjectRecord> {
    Ok(ProjectRecord {
        client: Some("acme".parse()?),
        role: Some("work".parse()?),
        billable: true,
        ..project(WEBSHOP_ID, "acme-webshop")?
    })
}

fn acme_catalog() -> Built<Catalog> {
    let mut garden = project(GARDEN_ID, "acme-garden")?;
    garden.client = Some("acme".parse()?);
    Ok(Catalog::new(
        vec![work()?],
        vec![acme()?],
        vec![webshop()?, garden],
    )?)
}

fn hourly_on(catalog: &Catalog, key: &str, on: &str) -> Built<Option<String>> {
    let project = catalog.project(&key.parse()?).ok_or("no such project")?;
    Ok(catalog
        .rate_on(project, day(on)?)
        .map(|rate| rate.hourly.to_string()))
}

#[rstest]
#[case::before_first_rate("acme-garden", "2024-12-31", None)]
#[case::first_rate_on_its_day("acme-garden", "2025-01-01", Some("95.00"))]
#[case::day_before_new_rate("acme-garden", "2026-06-30", Some("95.00"))]
#[case::new_rate_on_its_day("acme-garden", "2026-07-01", Some("105.00"))]
#[case::project_rate_before_clients("acme-webshop", "2026-09-01", Some("120.00"))]
#[case::client_rate_before_project_list("acme-webshop", "2026-08-31", Some("105.00"))]
#[case::client_rate_long_before_project_list("acme-webshop", "2025-03-01", Some("95.00"))]
fn rate_on_day(#[case] key: &str, #[case] on: &str, #[case] hourly: Option<&str>) -> TestResult {
    assert_eq!(hourly_on(&acme_catalog()?, key, on)?.as_deref(), hourly);
    Ok(())
}

#[test]
fn project_without_client_has_no_rate() -> TestResult {
    let catalog = Catalog::new(Vec::new(), Vec::new(), vec![project(GARDEN_ID, "garden")?])?;

    assert_eq!(hourly_on(&catalog, "garden", "2026-11-27")?, None);
    Ok(())
}

#[test]
fn rates_are_kept_sorted_by_day() -> TestResult {
    let rates = Rates::new(vec![rate("105", "2026-07-01")?, rate("95", "2025-01-01")?])?;

    let days: Vec<String> = rates.iter().map(|rate| rate.from.to_string()).collect();
    assert_eq!(days, ["2025-01-01", "2026-07-01"]);
    Ok(())
}

#[test]
fn two_rates_of_one_day_are_refused_and_named() -> TestResult {
    let error = Rates::new(vec![rate("95", "2026-07-01")?, rate("105", "2026-07-01")?]).err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("two rates begin on 2026-07-01: 95.00 and 105.00")
    );
    Ok(())
}

#[test]
fn project_with_unknown_client_names_closest_key() -> TestResult {
    let mut typo = webshop()?;
    typo.client = Some("acmee".parse()?);

    let error = Catalog::new(vec![work()?], vec![acme()?], vec![typo]).err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("the project `acme-webshop` names the client `acmee`, which pace doesn't know")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("did you mean `acme`?")
    );
    assert!(error.is_some_and(|error| error.is_code("pace::core::unknown-client")));
    Ok(())
}

#[test]
fn project_with_unknown_role_names_known_roles() -> TestResult {
    let mut typo = webshop()?;
    typo.role = Some("health".parse()?);

    let error = Catalog::new(vec![work()?], vec![acme()?], vec![typo]).err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("the project `acme-webshop` names the role `health`, which pace doesn't know")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("pace knows these roles: `work`")
    );
    Ok(())
}

#[test]
fn project_rates_for_unknown_project_name_closest_key() -> TestResult {
    let mut client = acme()?;
    client.project_rates = BTreeMap::from([(
        "acme-websop".parse()?,
        Rates::new(vec![rate("120", "2026-09-01")?])?,
    )]);

    let error = Catalog::new(vec![work()?], vec![client], vec![webshop()?]).err();

    assert_eq!(
        error.as_ref().map(ToString::to_string).as_deref(),
        Some("the client `acme` has rates for the project `acme-websop`, which pace doesn't know")
    );
    assert_eq!(
        error.as_ref().and_then(|error| error.hint()),
        Some("did you mean `acme-webshop`?")
    );
    Ok(())
}

#[test]
fn rate_in_other_currency_than_clients_is_refused() -> TestResult {
    let mut client = acme()?;
    let usd: Currency = "USD".parse()?;
    client.rates = Rates::new(vec![Rate {
        hourly: Money::parse_decimal("100", usd)?,
        from: day("2026-01-01")?,
    }])?;

    client.project_rates.clear();

    let error = Catalog::new(Vec::new(), vec![client], Vec::new()).err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("the client `acme` bills in EUR, but its rate from 2026-01-01 is 100.00 USD")
    );
    Ok(())
}

#[test]
fn two_projects_with_one_id_are_refused() -> TestResult {
    let error = Catalog::new(
        Vec::new(),
        Vec::new(),
        vec![project(GARDEN_ID, "garden")?, project(GARDEN_ID, "yard")?],
    )
    .err();

    assert_eq!(
        error.map(|error| error.to_string()),
        Some(format!(
            "the projects `garden` and `yard` have the same ID {GARDEN_ID}"
        ))
    );
    Ok(())
}

#[test]
fn two_projects_with_one_key_are_refused() -> TestResult {
    let error = Catalog::new(
        Vec::new(),
        Vec::new(),
        vec![
            project(GARDEN_ID, "garden")?,
            project(WEBSHOP_ID, "garden")?,
        ],
    )
    .err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("two projects have the key `garden`")
    );
    Ok(())
}

#[test]
fn two_roles_with_one_key_are_refused() -> TestResult {
    let mut other = work()?;
    other.id = GARDEN_ID.parse()?;

    let error = Catalog::new(vec![work()?, other], Vec::new(), Vec::new()).err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("two roles have the key `work`")
    );
    Ok(())
}

#[test]
fn two_roles_with_one_id_are_refused() -> TestResult {
    let mut other = work()?;
    other.key = "health".parse()?;

    let error = Catalog::new(vec![work()?, other], Vec::new(), Vec::new()).err();

    assert_eq!(
        error.map(|error| error.to_string()),
        Some(format!(
            "the roles `work` and `health` have the same ID {WORK_ID}"
        ))
    );
    Ok(())
}

#[test]
fn two_clients_with_one_key_are_refused() -> TestResult {
    let error = Catalog::new(Vec::new(), vec![acme()?, acme()?], Vec::new()).err();

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("two clients have the key `acme`")
    );
    Ok(())
}

#[test]
fn resolve_finds_project_by_key_and_by_id() -> TestResult {
    let catalog = acme_catalog()?;
    let id: ProjectId = WEBSHOP_ID.parse()?;

    let by_key = catalog.resolve(&ProjectRef::Key("acme-webshop".parse()?));
    let by_id = catalog.resolve(&ProjectRef::Id(id));

    assert_eq!(by_key.map(|project| project.id), Some(id));
    assert_eq!(
        by_id.map(|project| project.key.as_str()),
        Some("acme-webshop")
    );
    assert!(
        catalog
            .resolve(&ProjectRef::Key("deadbeef".parse()?))
            .is_none()
    );
    Ok(())
}

#[test]
fn project_role_resolves_its_key_to_role_id() -> TestResult {
    let catalog = acme_catalog()?;
    let webshop = catalog
        .project(&"acme-webshop".parse()?)
        .ok_or("no webshop")?;

    assert_eq!(
        catalog.role_of(webshop).map(|role| role.id),
        Some(WORK_ID.parse()?)
    );
    Ok(())
}

#[test]
fn lookups_find_role_and_client() -> TestResult {
    let catalog = acme_catalog()?;
    let work_id: RoleId = WORK_ID.parse()?;

    assert_eq!(
        catalog.role_by_id(work_id).map(|role| role.key.as_str()),
        Some("work")
    );
    assert_eq!(
        catalog.role(&"work".parse()?).map(|role| role.id),
        Some(work_id)
    );
    assert_eq!(
        catalog
            .client(&"acme".parse()?)
            .map(|client| client.name.as_str()),
        Some("ACME GmbH")
    );
    assert!(catalog.client(&"globex".parse()?).is_none());
    assert_eq!(catalog.projects().count(), 2);
    Ok(())
}

#[test]
fn empty_catalog_knows_nothing() -> TestResult {
    let catalog = Catalog::default();

    assert!(catalog.project_by_id(WEBSHOP_ID.parse()?).is_none());
    assert_eq!((catalog.roles().count(), catalog.clients().count()), (0, 0));
    Ok(())
}
