use pace_core::prelude::ActivityItem;
use pace_error::PaceResult;

use crate::entities::SQLiteActivityItem;

pub trait Convert {
    type Options;
    type Source;
    type Target;

    fn to_stored(source: Self::Source, opts: Self::Options) -> PaceResult<Self::Target>;
    fn from_stored(stored: Self::Target, opts: Self::Options) -> PaceResult<Self::Source>;
}

pub struct SQLiteActivityConverter;

impl Convert for SQLiteActivityConverter {
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
