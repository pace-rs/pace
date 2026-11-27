//! The questions of `pace role add`: the role's name, kind, purpose and
//! weekly target, each with a default.

use std::path::{Path, PathBuf};

use jiff::SignedDuration;
use pace_core::{Catalog, Role, RoleId, RoleKey, RoleKind};
use pace_error::{ErrorKind, ExitCode, PaceError, PaceResult};
use pace_store::{
    catalog::{ROLES_FILE, VERSION_KEY},
    field_edit::{FieldEdit, FieldValue},
};
use pace_time::parse_duration;

use crate::questions::{Answer, Ended, Flow, Optional, Question, QuestionKind, told};

/// The kinds of role, in the order the question offers them; `area` first,
/// as the default.
const KINDS: [RoleKind; 3] = [RoleKind::Area, RoleKind::AreaProject, RoleKind::Meta];

/// What the flags of `pace role add` gave, as typed; what they leave out
/// is asked. Each value passes its question's check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleFlags {
    /// `--name`
    pub name: Option<String>,
    /// `--kind`
    pub kind: Option<RoleKind>,
    /// `--purpose`
    pub purpose: Option<String>,
    /// `--weekly-target`, such as `30h`
    pub weekly_target: Option<String>,
}

/// The flow of a new role: it asks for what [`RoleFlags`] leave open and
/// ends with the role's fields in the roles file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddRoleFlow {
    roles_file: PathBuf,
    id: RoleId,
    key: RoleKey,
    flags: RoleFlags,
    name: Option<String>,
    kind: Option<RoleKind>,
    purpose: Optional<String>,
    weekly_target: Optional<SignedDuration>,
}

impl AddRoleFlow {
    /// The flow of the role `key` with the ID `id`, written to
    /// `roles_file`; `flags` answer their questions.
    ///
    /// # Errors
    ///
    /// If `key` is `version`, which the roles file keeps for its format
    /// (exit code 2), or if `catalog` has a role `key` already, archived or
    /// not.
    pub fn new(
        catalog: &Catalog,
        roles_file: &Path,
        key: RoleKey,
        id: RoleId,
        flags: RoleFlags,
    ) -> PaceResult<Self> {
        if key.to_string() == VERSION_KEY {
            return Err(PaceError::new(
                ErrorKind::InvalidInput,
                "a role can't have the key `{key}`, which {file} keeps for its format",
            )
            .attach_context("key", VERSION_KEY)
            .attach_context("file", ROLES_FILE)
            .attach_hint("give the role another key")
            .attach_exit_code(ExitCode::Usage));
        }
        if catalog.role(&key).is_some() {
            return Err(
                PaceError::new(ErrorKind::InvalidInput, "the role `{key}` exists already")
                    .attach_context("key", key.to_string())
                    .attach_hint("change it with `pace role edit`"),
            );
        }
        Ok(Self {
            roles_file: roles_file.to_path_buf(),
            id,
            key,
            flags,
            name: None,
            kind: None,
            purpose: Optional::Unasked,
            weekly_target: Optional::Unasked,
        })
    }
}

impl Flow for AddRoleFlow {
    type Made = Role;

    fn next_question(&self) -> Option<Question> {
        if self.name.is_none() {
            return Some(Question {
                id: "name",
                text: "What is the role's name?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(self.key.to_string()),
                },
                flag: "--name",
                flag_answer: self.flags.name.clone().map(Answer::Text),
                default_note: format!("name `{}` from the key", self.key),
            });
        }
        if self.kind.is_none() {
            return Some(Question {
                id: "kind",
                text: "What kind of role is it?".to_owned(),
                kind: QuestionKind::Choice {
                    items: KINDS.iter().map(|kind| kind_item(*kind)).collect(),
                    default: 0,
                },
                flag: "--kind",
                flag_answer: self.flags.kind.map(kind_answer),
                default_note: format!("kind `{}`", RoleKind::Area.as_str()),
            });
        }
        if self.purpose == Optional::Unasked {
            return Some(Question {
                id: "purpose",
                text: "What is the role for?".to_owned(),
                kind: QuestionKind::Text {
                    default: Some(String::new()),
                },
                flag: "--purpose",
                flag_answer: self.flags.purpose.clone().map(Answer::Text),
                default_note: "no purpose".to_owned(),
            });
        }
        (self.weekly_target == Optional::Unasked).then(|| Question {
            id: "weekly-target",
            text: "How much time should it get each week, such as 30h?".to_owned(),
            kind: QuestionKind::Text {
                default: Some(String::new()),
            },
            flag: "--weekly-target",
            flag_answer: self.flags.weekly_target.clone().map(Answer::Text),
            default_note: "no weekly target".to_owned(),
        })
    }

    fn take_answer(&mut self, answer: Answer) -> Result<(), String> {
        let Some(question) = self.next_question() else {
            return Err("every question has its answer".to_owned());
        };
        match (question.id, answer) {
            ("name", Answer::Text(name)) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err("a role needs a name".to_owned());
                }
                self.name = Some(name.to_owned());
            }
            ("kind", Answer::Choice(index)) => {
                let kind = KINDS.get(index).ok_or("pick one of the kinds")?;
                self.kind = Some(*kind);
            }
            ("purpose", Answer::Text(purpose)) => {
                let purpose = purpose.trim();
                self.purpose =
                    Optional::Answered((!purpose.is_empty()).then(|| purpose.to_owned()));
            }
            ("weekly-target", Answer::Text(target)) => {
                let target = target.trim();
                self.weekly_target = Optional::Answered(if target.is_empty() {
                    None
                } else {
                    Some(parse_duration(target).map_err(|error| told(&error))?)
                });
            }
            (id, answer) => return Err(format!("{answer:?} is no answer to `{id}`")),
        }
        Ok(())
    }

    fn finish(self) -> PaceResult<Ended<Role>> {
        let role = Role {
            id: self.id,
            key: self.key,
            kind: self.kind.unwrap_or(RoleKind::Area),
            name: self.name.unwrap_or_default(),
            purpose: self.purpose.into_value(),
            weekly_target: self.weekly_target.into_value(),
            archived: None,
        };
        Ok(Ended {
            edits: role_edits(&self.roles_file, &role),
            made: role,
        })
    }
}

/// The answer that picks `kind` among [`KINDS`].
fn kind_answer(kind: RoleKind) -> Answer {
    Answer::Choice(KINDS.iter().position(|known| *known == kind).unwrap_or(0))
}

/// How the kind question shows `kind`: its word and what it means.
fn kind_item(kind: RoleKind) -> String {
    let meaning = match kind {
        RoleKind::Area => "a lasting responsibility, such as work or health",
        RoleKind::AreaProject => "an area that keeps producing projects",
        RoleKind::Meta => "planning and reviewing your time itself",
    };
    format!("{}: {meaning}", kind.as_str())
}

/// The field edits that write `role` as its table in `roles_file`.
fn role_edits(roles_file: &Path, role: &Role) -> Vec<FieldEdit> {
    let key = role.key.to_string();
    let field = |name: &str, value: FieldValue| FieldEdit::set(roles_file, &[&key, name], value);
    let mut edits = vec![
        field("id", FieldValue::String(role.id.to_string())),
        field("kind", FieldValue::String(role.kind.as_str().to_owned())),
        field("name", FieldValue::String(role.name.clone())),
    ];
    if let Some(purpose) = &role.purpose {
        edits.push(field("purpose", FieldValue::String(purpose.clone())));
    }
    if let Some(target) = role.weekly_target {
        edits.push(field(
            "weekly-target",
            FieldValue::String(format!("{target:#}")),
        ));
    }
    edits
}

#[cfg(test)]
mod tests {
    use pace_error::ExitCode;

    use super::*;
    use crate::prompt::Scripted;
    use crate::questions::ask_flow;

    const ID: &str = "0193a7c4-0000-7000-8000-000000000001";

    fn flow(flags: RoleFlags) -> AddRoleFlow {
        AddRoleFlow::new(
            &Catalog::default(),
            Path::new("roles.toml"),
            "work".parse().unwrap(),
            ID.parse().unwrap(),
            flags,
        )
        .unwrap()
    }

    fn set(key: &str, value: &str) -> FieldEdit {
        FieldEdit::set(
            "roles.toml",
            &["work", key],
            FieldValue::String(value.to_owned()),
        )
    }

    #[test]
    fn every_flag_given_asks_nothing() {
        let flags = RoleFlags {
            name: Some("Work".to_owned()),
            kind: Some(RoleKind::AreaProject),
            purpose: Some("Earn a living".to_owned()),
            weekly_target: Some("30h".to_owned()),
        };
        let asker = Scripted::without_terminal();

        let result = ask_flow(flow(flags), &asker).unwrap();

        assert_eq!(
            result.edits,
            [
                set("id", ID),
                set("kind", "area-project"),
                set("name", "Work"),
                set("purpose", "Earn a living"),
                set("weekly-target", "30h"),
            ]
        );
        assert_eq!(result.defaults, Vec::<String>::new());
    }

    #[test]
    fn no_terminal_takes_defaults_and_names_them() {
        let result = ask_flow(flow(RoleFlags::default()), &Scripted::without_terminal()).unwrap();

        assert_eq!(
            result.edits,
            [set("id", ID), set("kind", "area"), set("name", "work")]
        );
        assert_eq!(
            result.defaults,
            [
                "name `work` from the key",
                "kind `area`",
                "no purpose",
                "no weekly target"
            ]
        );
    }

    #[test]
    fn terminal_asks_what_flags_leave_open() {
        let flags = RoleFlags {
            name: Some("Health".to_owned()),
            ..RoleFlags::default()
        };
        let asker = Scripted::typing(["Keep fit", "4h"]).and_choosing([2]);

        let result = ask_flow(flow(flags), &asker).unwrap();

        assert_eq!(result.made.kind, RoleKind::Meta);
        assert_eq!(result.made.purpose.as_deref(), Some("Keep fit"));
        assert_eq!(
            result.made.weekly_target,
            Some(SignedDuration::from_hours(4))
        );
        assert_eq!(
            asker.questions(),
            [
                "What kind of role is it?",
                "What is the role for?",
                "How much time should it get each week, such as 30h?"
            ]
        );
        assert_eq!(
            asker.offered(),
            [[
                "area: a lasting responsibility, such as work or health",
                "area-project: an area that keeps producing projects",
                "meta: planning and reviewing your time itself"
            ]]
        );
    }

    #[test]
    fn weekly_target_that_is_no_duration_asks_again() {
        let asker = Scripted::typing(["", "", "thirty", "30h"]).and_choosing([0]);

        let result = ask_flow(flow(RoleFlags::default()), &asker).unwrap();

        assert_eq!(
            result.made.weekly_target,
            Some(SignedDuration::from_hours(30))
        );
        assert_eq!(
            asker.told(),
            ["`thirty` is not a duration: write it like `15m`, `1h30m` or `1.5h`"]
        );
    }

    #[test]
    fn blank_name_asks_again() {
        let asker = Scripted::typing(["  ", "Work", "", ""]).and_choosing([0]);

        let result = ask_flow(flow(RoleFlags::default()), &asker).unwrap();

        assert_eq!(result.made.name, "Work");
        assert_eq!(asker.told(), ["a role needs a name"]);
    }

    #[test]
    fn blank_name_flag_is_usage_error_naming_flag() {
        let flags = RoleFlags {
            name: Some("  ".to_owned()),
            ..RoleFlags::default()
        };

        let error = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "`--name \"  \"` doesn't fit: a role needs a name"
        );
    }

    #[test]
    fn name_and_purpose_flags_are_trimmed() {
        let flags = RoleFlags {
            name: Some("  Home  ".to_owned()),
            purpose: Some("  ".to_owned()),
            ..RoleFlags::default()
        };

        let result = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap();

        assert_eq!(result.made.name, "Home");
        assert_eq!(result.made.purpose, None);
        assert_eq!(result.defaults, ["kind `area`", "no weekly target"]);
    }

    #[test]
    fn weekly_target_flag_that_is_no_duration_is_usage_error() {
        let flags = RoleFlags {
            weekly_target: Some("thirty".to_owned()),
            ..RoleFlags::default()
        };

        let error = ask_flow(flow(flags), &Scripted::without_terminal()).unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.hint(),
            Some("give `--weekly-target` another value, or leave the flag out")
        );
    }

    #[test]
    fn key_version_is_refused() {
        let error = AddRoleFlow::new(
            &Catalog::default(),
            Path::new("roles.toml"),
            "version".parse().unwrap(),
            ID.parse().unwrap(),
            RoleFlags::default(),
        )
        .unwrap_err();

        assert_eq!(error.exit_code(), ExitCode::Usage);
        assert_eq!(
            error.to_string(),
            "a role can't have the key `version`, which roles.toml keeps for its format"
        );
        assert_eq!(error.hint(), Some("give the role another key"));
    }

    #[test]
    fn existing_key_names_role_edit() {
        let role = Role {
            id: ID.parse().unwrap(),
            key: "work".parse().unwrap(),
            kind: RoleKind::Area,
            name: "Work".to_owned(),
            purpose: None,
            weekly_target: None,
            archived: None,
        };
        let catalog = Catalog::new(vec![role], Vec::new(), Vec::new()).unwrap();

        let error = AddRoleFlow::new(
            &catalog,
            Path::new("roles.toml"),
            "work".parse().unwrap(),
            "0193a7c4-0000-7000-8000-000000000002".parse().unwrap(),
            RoleFlags::default(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "the role `work` exists already");
        assert_eq!(error.hint(), Some("change it with `pace role edit`"));
        assert_eq!(error.exit_code(), ExitCode::Error);
    }
}
