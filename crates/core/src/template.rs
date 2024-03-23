use lazy_static::lazy_static;
use tera::Tera;

use crate::prelude::{ReflectionSummary, SummaryActivityGroup};

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
        // tera.register_filter("do_nothing", do_nothing_filter);
        tera
    };
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

        context.insert("total_time_spent", &value.total_time_spent());
        context.insert("total_break_duration", &value.total_break_duration());

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
            .collect::<std::collections::HashMap<String, &SummaryActivityGroup>>();

        context.insert("summary_groups_by_category", &summary_groups_by_category);

        Self { context }
    }
}
