//! The known name closest to a mistyped one, for a "did you mean" hint.

/// The similarity a known name must exceed to count as meant (Jaro-Winkler).
const THRESHOLD: f64 = 0.8;

/// The known name closest to `input`, if any is close enough to be a typo.
///
/// The score is the Jaro-Winkler similarity; a name must score above 0.8. Of
/// names with equal scores, the last one wins.
///
/// ```
/// use pace_error::closest_name;
///
/// let known = ["version", "projects"];
/// assert_eq!(closest_name("verison", known.into_iter()), Some("version"));
/// assert_eq!(closest_name("zzz", known.into_iter()), None);
/// ```
pub fn closest_name<'names>(
    input: &str,
    names: impl Iterator<Item = &'names str>,
) -> Option<&'names str> {
    names
        .map(|name| (strsim::jaro_winkler(input, name), name))
        .filter(|(score, _)| *score > THRESHOLD)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, name)| name)
}
