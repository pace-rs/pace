/// Overwrite any value with another.
///
/// This can be used to overwrite an activity with another activity.
///
/// # Arguments
///
/// * `left` - The left value
/// * `right` - The right value
pub fn overwrite<T>(left: &mut T, right: T) {
    *left = right;
}
