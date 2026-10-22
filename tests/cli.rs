//! Regression tests for the failures that motivated the rewrite.
//!
//! These drive the real `ctm` binary through stdin/stdout, so they fail if the
//! EOF handling, name handling, validation or exit codes regress.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn scratch_dir(label: &str) -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ctm-cli-{}-{}-{}",
        std::process::id(),
        label,
        unique
    ));
    fs::create_dir_all(&path).expect("create scratch dir");
    path
}

fn ctm(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ctm"));
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("spawn ctm");
    if let Some(input) = stdin {
        let mut pipe = child.stdin.take().expect("stdin pipe");
        pipe.write_all(input.as_bytes()).expect("write stdin");
        // Dropping the pipe sends EOF, which is what the old CLI mishandled.
    }
    child.wait_with_output().expect("wait for ctm")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write_players(dir: &Path, content: &str) -> PathBuf {
    let path = dir.join("players.txt");
    fs::write(&path, content).expect("write players");
    path
}

fn new_tournament(dir: &Path, players: &str) -> PathBuf {
    let players_path = write_players(dir, players);
    let tournament = dir.join("cup.json");
    let output = ctm(
        &[
            "new",
            "--name",
            "Cup",
            "--players",
            players_path.to_str().unwrap(),
            "--out",
            tournament.to_str().unwrap(),
        ],
        None,
    );
    assert!(
        output.status.success(),
        "new failed: {}",
        stderr_of(&output)
    );
    tournament
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read file");
    serde_json::from_str(&raw).expect("valid json")
}

#[test]
fn rejects_a_non_numeric_rating() {
    let dir = scratch_dir("rating");
    let players = write_players(&dir, "Alice:abc\nBob:1500\n");
    let output = ctm(
        &[
            "new",
            "--name",
            "Cup",
            "--players",
            players.to_str().unwrap(),
            "--out",
            dir.join("cup.json").to_str().unwrap(),
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(2), "wrong exit code");
    let stderr = stderr_of(&output);
    assert!(stderr.contains("Alice"), "stderr: {stderr}");
    assert!(stderr.contains("not a number"), "stderr: {stderr}");
    assert!(stderr.contains("players.txt:1"), "stderr: {stderr}");

    fs::remove_dir_all(dir).ok();
}

#[test]
fn rejects_an_ambiguous_player_spec() {
    let dir = scratch_dir("spec");
    let players = write_players(&dir, "Smith: Jr:1400\nBob:1500\n");
    let output = ctm(
        &[
            "new",
            "--name",
            "Cup",
            "--players",
            players.to_str().unwrap(),
            "--out",
            dir.join("cup.json").to_str().unwrap(),
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(2), "wrong exit code");
    assert!(
        stderr_of(&output).contains("unexpected ':'"),
        "stderr: {}",
        stderr_of(&output)
    );

    fs::remove_dir_all(dir).ok();
}

#[test]
fn rejects_duplicate_names() {
    let dir = scratch_dir("duplicate");
    let players = write_players(&dir, "Alice:1500\nalice:1600\n");
    let output = ctm(
        &[
            "new",
            "--name",
            "Cup",
            "--players",
            players.to_str().unwrap(),
            "--out",
            dir.join("cup.json").to_str().unwrap(),
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(2), "wrong exit code");
    assert!(
        stderr_of(&output).to_lowercase().contains("duplicate"),
        "stderr: {}",
        stderr_of(&output)
    );

    fs::remove_dir_all(dir).ok();
}

#[test]
fn eof_saves_and_exits_instead_of_hanging() {
    let dir = scratch_dir("eof");
    let tournament = new_tournament(&dir, "Alice:1500\nBob:1600\n");

    let started = Instant::now();
    let output = ctm(&["-f", tournament.to_str().unwrap(), "run"], None);
    let elapsed = started.elapsed();

    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    assert!(
        elapsed < Duration::from_secs(10),
        "run did not stop on EOF (took {elapsed:?})"
    );

    let json = read_json(&tournament);
    assert_eq!(json["tournament"]["rounds"].as_array().unwrap().len(), 1);
    assert!(json["tournament"]["rounds"][0]["games"][0]["result"].is_null());

    fs::remove_dir_all(dir).ok();
}

#[test]
fn angle_bracket_bye_is_an_ordinary_player() {
    let dir = scratch_dir("bye");
    let tournament = new_tournament(&dir, "<bye>:1500\nBob:1600\nCarol:1700\n");

    // Three players, three rounds, one game per round.
    let output = ctm(
        &["-f", tournament.to_str().unwrap(), "run"],
        Some("1-0\n1-0\n1-0\n"),
    );
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let json = read_json(&tournament);
    let rounds = json["tournament"]["rounds"].as_array().unwrap();
    assert_eq!(rounds.len(), 3);
    let games: usize = rounds
        .iter()
        .map(|round| round["games"].as_array().unwrap().len())
        .sum();
    assert_eq!(games, 3, "every round should have exactly one game");

    let standings = ctm(
        &["-f", tournament.to_str().unwrap(), "standings", "--json"],
        None,
    );
    assert!(standings.status.success(), "stderr: {}", stderr_of(&standings));

    let rows: Value = serde_json::from_str(&stdout_of(&standings)).expect("standings json");
    let rows = rows.as_array().expect("array of rows");
    assert_eq!(rows.len(), 3, "all three players must be ranked");
    let names: Vec<&str> = rows
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"<bye>"), "{names:?}");
    assert!(names.contains(&"Bob"), "{names:?}");
    assert!(names.contains(&"Carol"), "{names:?}");

    fs::remove_dir_all(dir).ok();
}

#[test]
fn results_can_be_entered_corrected_and_undone() {
    let dir = scratch_dir("results");
    let tournament = new_tournament(&dir, "Alice:1500\nBob:1600\n");

    let file = tournament.to_str().unwrap();
    let run = ctm(&["-f", file, "run"], None);
    assert!(run.status.success(), "stderr: {}", stderr_of(&run));

    let set = ctm(&["-f", file, "result", "--round", "1", "--board", "1", "0-1"], None);
    assert!(set.status.success(), "stderr: {}", stderr_of(&set));

    let json = read_json(&tournament);
    assert_eq!(
        json["tournament"]["rounds"][0]["games"][0]["result"],
        Value::String("0-1".into())
    );

    // Correct it to a draw, then undo it entirely.
    let set = ctm(&["-f", file, "result", "--round", "1", "--board", "1", "1/2-1/2"], None);
    assert!(set.status.success(), "stderr: {}", stderr_of(&set));
    let json = read_json(&tournament);
    assert_eq!(
        json["tournament"]["rounds"][0]["games"][0]["result"],
        Value::String("1/2-1/2".into())
    );

    let undo = ctm(&["-f", file, "undo", "--round", "1", "--board", "1"], None);
    assert!(undo.status.success(), "stderr: {}", stderr_of(&undo));
    let json = read_json(&tournament);
    assert!(json["tournament"]["rounds"][0]["games"][0]["result"].is_null());

    // Reject nonsense instead of silently accepting it.
    let bad = ctm(&["-f", file, "result", "--round", "1", "--board", "1", "2"], None);
    assert_eq!(bad.status.code(), Some(1), "stderr: {}", stderr_of(&bad));

    fs::remove_dir_all(dir).ok();
}

#[test]
fn a_complete_tournament_ranks_every_player_once() {
    let dir = scratch_dir("complete");
    let tournament = new_tournament(&dir, "Alice:1700\nBob:1600\nCarol:1500\nDave:1400\n");

    let output = ctm(
        &["-f", tournament.to_str().unwrap(), "run"],
        Some("1-0\n0-1\n1/2-1/2\n1-0\n0-1\n1/2-1/2\n"),
    );
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let json = read_json(&tournament);
    let rounds = json["tournament"]["rounds"].as_array().unwrap();
    assert_eq!(rounds.len(), 3);
    for round in rounds {
        assert_eq!(round["games"].as_array().unwrap().len(), 2);
        assert!(round["bye"].is_null());
    }

    let standings = ctm(
        &["-f", tournament.to_str().unwrap(), "standings", "--json"],
        None,
    );
    let rows: Value = serde_json::from_str(&stdout_of(&standings)).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 4);
    assert_eq!(rows[0]["rank"], 1);

    fs::remove_dir_all(dir).ok();
}

#[test]
fn piped_output_has_no_escape_codes_unless_asked() {
    let dir = scratch_dir("colour");
    let tournament = new_tournament(&dir, "Alice:1500\nBob:1600\n");

    let plain = ctm(&["-f", tournament.to_str().unwrap(), "standings"], None);
    assert!(!stdout_of(&plain).contains('\u{1b}'), "piped output was colourised");

    let forced = ctm(
        &["--color", "always", "-f", tournament.to_str().unwrap(), "standings"],
        None,
    );
    assert!(stdout_of(&forced).contains('\u{1b}'), "forced colour was stripped");

    fs::remove_dir_all(dir).ok();
}

#[test]
fn exports_pgn_and_csv() {
    let dir = scratch_dir("export");
    let tournament = new_tournament(&dir, "Alice:1500\nBob:1600\n");
    let file = tournament.to_str().unwrap();

    ctm(&["-f", file, "run"], None);
    ctm(&["-f", file, "result", "--round", "1", "--board", "1", "1-0"], None);

    let pgn = ctm(&["-f", file, "export", "pgn"], None);
    assert!(pgn.status.success(), "stderr: {}", stderr_of(&pgn));
    assert!(stdout_of(&pgn).contains("[Result \"1-0\"]"));

    let csv = ctm(&["-f", file, "export", "csv"], None);
    assert!(csv.status.success(), "stderr: {}", stderr_of(&csv));
    assert!(stdout_of(&csv).starts_with("rank,player,rating"));

    fs::remove_dir_all(dir).ok();
}

#[test]
fn missing_file_is_a_usage_error() {
    let output = ctm(&["standings"], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("no tournament file"));
}

#[test]
fn corrupt_save_file_is_reported_not_ignored() {
    let dir = scratch_dir("corrupt");
    let path = dir.join("broken.json");
    fs::write(&path, "{\"version\":1,\"tournament\":{}}").unwrap();

    let output = ctm(&["-f", path.to_str().unwrap(), "standings"], None);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("broken.json"),
        "stderr: {}",
        stderr_of(&output)
    );

    fs::remove_dir_all(dir).ok();
}
