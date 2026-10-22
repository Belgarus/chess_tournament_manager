//! Round-robin tournament manager.
//!
//! The library is split so that the rules are independent of the interface:
//!
//! - [`model`] holds the immutable facts: players, rounds, games, results.
//! - [`pairing`] turns that history into the next round.
//! - [`standings`] and [`tiebreak`] derive the table from the history.
//! - [`storage`] and [`export`] move tournaments in and out of files.
//! - [`ui`] and [`cli`] are the only modules that know about terminals.

pub mod cli;
pub mod error;
pub mod export;
pub mod model;
pub mod pairing;
pub mod standings;
pub mod storage;
pub mod tiebreak;
pub mod ui;
