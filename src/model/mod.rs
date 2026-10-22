//! Domain model for a round-robin tournament.
//!
//! The tournament is stored as raw facts only: players plus the rounds that
//! were actually paired, with one optional result per game. Standings, scores,
//! Buchholz, colour balance and win/loss records are all *derived* from that
//! history, so a result can always be corrected without the counters drifting.

pub mod game;
pub mod player;
pub mod round;
pub mod tournament;

pub use game::{Color, ColorHistory, Game, Outcome};
pub use player::{validate_spec, Player, PlayerId, DEFAULT_RATING, MAX_RATING, NAME_MAX_CHARS};
pub use round::Round;
pub use tournament::Tournament;
