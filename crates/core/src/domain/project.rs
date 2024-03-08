use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    path::PathBuf,
};

use serde_derive::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use ulid::Ulid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectList {
    /// The tasks in the list
    #[serde(flatten)]
    projects: BTreeMap<ProjectGuid, Project>,
    defaults: Option<DefaultOptions>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DefaultOptions {
    categories: Option<Vec<Category>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Category {
    id: Ulid,
    name: String,
    description: Option<String>,
}

#[derive(Debug, TypedBuilder, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    tasks_file: PathBuf,

    filters: Option<Vec<String>>,

    #[serde(flatten)]
    subproject_options: Option<SubprojectOptions>,
}

#[derive(Debug, TypedBuilder, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct SubprojectOptions {
    parent_id: Option<Ulid>,
}

/// The unique identifier of an activity
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialEq, PartialOrd, Eq, Copy, Hash)]
pub struct ProjectGuid(Ulid);

impl Display for ProjectGuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for ProjectGuid {
    fn default() -> Self {
        Self(Ulid::new())
    }
}

#[cfg(test)]
mod tests {

    use crate::error::TestResult;

    use super::*;
    use rstest::*;
    use std::{fs, path::PathBuf};

    #[rstest]
    fn test_parse_project_file_passes(
        #[files("../../config/projects.pace.toml")] config_path: PathBuf,
    ) -> TestResult<()> {
        let toml_string = fs::read_to_string(config_path)?;
        let _ = toml::from_str::<ProjectList>(&toml_string)?;

        Ok(())
    }
}
