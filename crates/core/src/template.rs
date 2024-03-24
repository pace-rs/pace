use std::collections::HashMap;

use lazy_static::lazy_static;
use pace_time::duration::PaceDuration;
use tera::{from_value, to_value, Error, Tera, Value};

use crate::domain::reflection::{ReflectionSummary, SummaryActivityGroup};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/reflections/**") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {e}");
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera.register_filter("human_duration", human_duration);
        tera
    };
}

/// Returns the human duration of the argument.
pub fn human_duration(value: &Value, _: &HashMap<String, Value>) -> Result<Value, Error> {
    let Ok(duration) = from_value::<PaceDuration>(value.clone()) else {
        return Err(Error::msg(format!(
            "Function `human-duration` received an invalid argument: `{value:?}`"
        )));
    };

    to_value(duration.human_readable()).map_err(Error::json)
}

#[derive(Debug)]
pub struct PaceReflectionTemplate {
    context: tera::Context,
}

impl PaceReflectionTemplate {
    pub fn into_context(self) -> tera::Context {
        self.context
    }
}

impl From<ReflectionSummary> for PaceReflectionTemplate {
    fn from(value: ReflectionSummary) -> Self {
        let mut context = tera::Context::new();
        context.insert("time_range_start", &value.time_range().start());
        context.insert("time_range_end", &value.time_range().end());

        context.insert(
            "total_time_spent",
            &value.total_time_spent().human_readable(),
        );
        context.insert(
            "total_break_duration",
            &value.total_break_duration().human_readable(),
        );

        // key must be a string, because of the way tera works with nested objects
        // we need to convert the key to a string

        // merge key tuples into a single string
        let summary_groups_by_category = value
            .summary_groups_by_category()
            .iter()
            .map(|((category, subcategory), summary_group)| {
                let key = format!("{category}::{subcategory}");
                (key, summary_group)
            })
            .collect::<HashMap<String, &SummaryActivityGroup>>();

        context.insert("summary_groups_by_category", &summary_groups_by_category);

        Self { context }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_template_filter_human_duration_passes() -> Result<(), Error> {
        let value = 31_651_469;

        let print_duration = human_duration(&to_value(value)?, &HashMap::default())?;

        assert_eq!(print_duration, to_value("1year 1day 2h 4m 29s")?);

        Ok(())
    }
}
