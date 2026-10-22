use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::model::Tournament;

pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    tournament: Tournament,
}

/// Write the tournament atomically: a temporary sibling file is written first
/// and then renamed over the target, so an interrupted save cannot leave a
/// half-written tournament behind.
pub fn save(tournament: &Tournament, path: &Path) -> Result<()> {
    let payload = SaveFile {
        version: SAVE_VERSION,
        tournament: tournament.clone(),
    };
    let json = serde_json::to_string_pretty(&payload)?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
        }
    }

    let temporary = temporary_path(path);
    fs::write(&temporary, json.as_bytes()).map_err(|error| AppError::io(&temporary, error))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        AppError::io(path, error)
    })
}

pub fn load(path: &Path) -> Result<Tournament> {
    let raw = fs::read_to_string(path).map_err(|error| AppError::io(path, error))?;
    let file: SaveFile = serde_json::from_str(&raw)
        .map_err(|error| AppError::parse(path, error.line(), error.to_string()))?;

    if file.version != SAVE_VERSION {
        return Err(AppError::usage(format!(
            "{}: unsupported file version {} (this build understands version {})",
            path.display(),
            file.version,
            SAVE_VERSION
        )));
    }

    file.tournament.validate()?;
    Ok(file.tournament)
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("tournament.json");
    path.with_file_name(format!(".{name}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Outcome, Player};

    fn sample() -> Tournament {
        let players = vec![
            Player::new(0, "Alice", 1700),
            Player::new(1, "Bob", 1500),
        ];
        let mut tournament = Tournament::new("Sample".into(), players, false);
        tournament.pair_next().unwrap();
        tournament.set_result(1, 1, Outcome::WhiteWin).unwrap();
        tournament
    }

    fn scratch_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("ctm-storage-{}-{name}.json", std::process::id()));
        path
    }

    #[test]
    fn round_trips_a_tournament() {
        let path = scratch_path("roundtrip");
        let original = sample();
        save(&original, &path).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.players, original.players);
        assert_eq!(loaded.rounds.len(), 1);
        assert_eq!(
            loaded.rounds[0].games[0].result,
            Some(Outcome::WhiteWin)
        );

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn rejects_broken_json() {
        let path = scratch_path("broken");
        fs::write(&path, "{ not json").unwrap();
        let error = load(&path).unwrap_err();
        assert!(matches!(error, AppError::Parse { .. }), "{error}");
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn rejects_unknown_version() {
        let path = scratch_path("version");
        fs::write(&path, r#"{"version":99,"tournament":{"name":"x","double_round_robin":false,"players":[],"rounds":[]}}"#).unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.to_string().contains("unsupported file version"), "{error}");
        fs::remove_file(&path).unwrap();
    }
}
