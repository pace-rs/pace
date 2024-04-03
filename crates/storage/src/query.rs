use pace_error::PaceResult;

// Lazy loading related entities
trait LazyLoad {
    fn query(&self) -> PaceResult<Vec<Self>>
    where
        Self: Sized;
}
