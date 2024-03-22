pub mod date;
pub mod date_time;
pub mod duration;
pub mod error;
pub mod flags;
pub mod time;
pub mod time_frame;
pub mod time_range;
pub mod time_zone;

pub trait Validate {
    type Output;
    type Error;

    /// Validate a struct
    ///
    /// # Errors
    ///
    /// Returns an error if the validation was not successful
    ///
    /// # Returns
    ///
    /// Returns the struct if the validation was successful
    fn validate(self) -> Result<Self::Output, Self::Error>;
}
