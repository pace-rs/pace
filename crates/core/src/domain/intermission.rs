//! Intermission entity and business logic

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::domain::{status::ItemStatus, tag::Tag, task::TaskList};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntermissionPeriod {
    start_date: NaiveDate,
    start_time: NaiveTime,
    end_date: Option<NaiveDate>,
    end_time: Option<NaiveTime>,
}

impl IntermissionPeriod {
    pub fn new(start_date: NaiveDate, start_time: NaiveTime) -> Self {
        Self {
            start_date,
            start_time,
            end_date: None,
            end_time: None,
        }
    }

    pub fn end(&mut self, end_date: NaiveDate, end_time: NaiveTime) {
        self.end_date = Some(end_date);
        self.end_time = Some(end_time);
    }
}
