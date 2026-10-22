use serde::{Deserialize, Serialize};

pub const DEFAULT_RATING: u32 = 1200;
pub const MAX_RATING: u32 = 4000;
pub const NAME_MAX_CHARS: usize = 40;

/// Stable handle for a player. Player ids are their index in
/// [`crate::model::Tournament::players`], which makes them cheap and, more
/// importantly, means no player name is ever special.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct PlayerId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub rating: u32,
}

impl Player {
    pub fn new(id: usize, name: impl Into<String>, rating: u32) -> Self {
        Self {
            id: PlayerId(id),
            name: name.into(),
            rating,
        }
    }
}

/// Parse one `name[:rating]` entry, rejecting anything ambiguous instead of
/// silently guessing. Returns a human-readable message on failure so callers
/// can attach the file and line number.
///
/// The old UI split on every `:` and fell back to the default rating when the
/// number failed to parse, which turned `Smith: Jr:1400` into `Smith` rated
/// 1200 without a word. Here a second colon is an error.
pub fn validate_spec(spec: &str, default_rating: u32) -> std::result::Result<(String, u32), String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("empty player entry".to_string());
    }

    let (name_raw, rating_raw) = match spec.split_once(':') {
        Some((name, rating)) => (name, Some(rating)),
        None => (spec, None),
    };

    let name = name_raw.trim();
    if name.is_empty() {
        return Err(format!("player name is empty in {spec:?}"));
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return Err(format!(
            "player name is longer than {NAME_MAX_CHARS} characters: {name:?}"
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(format!("player name contains control characters: {name:?}"));
    }

    let rating = match rating_raw {
        None => default_rating,
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                default_rating
            } else if raw.contains(':') {
                return Err(format!(
                    "unexpected ':' in the rating of {name:?} (expected name[:rating]): {raw:?}"
                ));
            } else {
                let parsed: u32 = raw
                    .parse()
                    .map_err(|_| format!("rating of {name:?} is not a number: {raw:?}"))?;
                if parsed > MAX_RATING {
                    return Err(format!(
                        "rating of {name:?} is out of range (0..={MAX_RATING}): {parsed}"
                    ));
                }
                parsed
            }
        }
    };

    Ok((name.to_string(), rating))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_name_only() {
        assert_eq!(validate_spec("Alice", 1200), Ok(("Alice".into(), 1200)));
    }

    #[test]
    fn accepts_explicit_rating() {
        assert_eq!(validate_spec(" Alice : 1850 ", 1200), Ok(("Alice".into(), 1850)));
    }

    #[test]
    fn empty_rating_uses_default() {
        assert_eq!(validate_spec("Alice:", 1300), Ok(("Alice".into(), 1300)));
    }

    #[test]
    fn rejects_non_numeric_rating() {
        let err = validate_spec("Alice:abc", 1200).unwrap_err();
        assert!(err.contains("not a number"), "{err}");
    }

    #[test]
    fn rejects_second_colon() {
        let err = validate_spec("Smith: Jr:1400", 1200).unwrap_err();
        assert!(err.contains("unexpected ':'"), "{err}");
    }

    #[test]
    fn rejects_out_of_range_rating() {
        let err = validate_spec("Alice:9999", 1200).unwrap_err();
        assert!(err.contains("out of range"), "{err}");
    }

    #[test]
    fn rejects_empty_name() {
        assert!(validate_spec(":1500", 1200).is_err());
    }

    #[test]
    fn bye_is_not_special() {
        assert_eq!(validate_spec("<bye>:1500", 1200), Ok(("<bye>".into(), 1500)));
    }
}
