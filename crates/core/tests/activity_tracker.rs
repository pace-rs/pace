//! Test the ActivityStore implementation with a InMemoryStorage backend.

use std::{collections::HashSet, sync::Arc};

use chrono::{Local, NaiveDateTime};

use pace_core::{
    Activity, ActivityGuid, ActivityItem, ActivityKind, ActivityKindOptions, ActivityLog,
    ActivityReadOps, ActivityStateManagement, ActivityStatus, ActivityStatusFilter, ActivityStore,
    ActivityWriteOps, DeleteOptions, EndOptions, HoldOptions, InMemoryActivityStorage,
    PaceDateTime, ResumeOptions, TestResult, UpdateOptions,
};
use rstest::{fixture, rstest};
use similar_asserts::assert_eq;
