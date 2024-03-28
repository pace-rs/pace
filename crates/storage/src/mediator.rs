use pace_core::prelude::ActivityItem;
use pace_error::PaceResult;

use crate::entities::SQLiteActivityItem;

pub trait Mediator {
    type Options;
    type Source;
    type Target;

    fn to_stored(source: Self::Source, opts: Self::Options) -> PaceResult<Self::Target>;
    fn from_stored(stored: Self::Target, opts: Self::Options) -> PaceResult<Self::Source>;
}

pub struct SQLiteActivityMediator;

impl Mediator for SQLiteActivityMediator {
    type Options = ();
    type Source = ActivityItem;
    type Target = SQLiteActivityItem;

    fn to_stored(source: Self::Source, opts: Self::Options) -> PaceResult<Self::Target> {
        todo!()
    }

    fn from_stored(stored: Self::Target, opts: Self::Options) -> PaceResult<Self::Source> {
        todo!()
    }
}
