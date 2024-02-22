/// Overwrite any value with another.
///
/// This can be used to overwrite an activity with another activity.
///
/// # Arguments
///
/// * `left` - The left value
/// * `right` - The right value
pub fn overwrite_left_with_right<T>(left: &mut T, right: T) {
    *left = right;
}

#[cfg(test)]
mod tests {

    use crate::Activity;

    use super::*;

    #[test]
    fn test_overwrite_i32_passes() {
        let mut left = 1;
        let right = 2;
        overwrite_left_with_right(&mut left, right);
        assert_eq!(left, 2);
    }

    #[test]
    fn test_overwrite_string_passes() {
        let mut left = String::from("left");
        let right = String::from("right");
        overwrite_left_with_right(&mut left, right);
        assert_eq!(left, "right");
    }

    #[test]
    fn test_overwrite_activity_passes() {
        let mut left = Activity::default();
        let mut right = Activity::default();
        _ = right.category_mut().replace("right".to_string());
        overwrite_left_with_right(&mut left, right);
        assert_eq!(left.category(), &Some("right".to_string()));
    }
}
