//! Intermission entity and business logic

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::domain::{status::ItemStatus, tag::Tag, task::TaskList, time::PaceDuration};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct IntermissionPeriod {
    begin: NaiveDateTime,
    end: Option<NaiveDateTime>,
    duration: Option<PaceDuration>,
}

impl Default for IntermissionPeriod {
    fn default() -> Self {
        Self {
            begin: Local::now().naive_local(),
            end: None,
            duration: None,
        }
    }
}

impl IntermissionPeriod {
    pub fn new(
        begin: NaiveDateTime,
        end: Option<NaiveDateTime>,
        duration: Option<PaceDuration>,
    ) -> Self {
        Self {
            begin,
            end,
            duration,
        }
    }

    pub fn end(&mut self, end: NaiveDateTime) {
        // TODO!: Calculate duration
        self.end = Some(end);
    }
}
