use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use crate::error::{AppError, Result};
use crate::export;
use crate::model::{validate_spec, Outcome, Player, Round, Tournament, DEFAULT_RATING};
use crate::storage;
use crate::tiebreak::Tiebreak;
use crate::ui::{prompt, table, ColorChoice, Painter};

#[derive(Debug, Parser)]
#[command(
    name = "ctm",
    version,
    about = "Round-robin chess tournament manager",
    after_help = "Examples:\n  ctm new --name \"Club Cup\" --players players.txt\n  ctm -f club-cup.json run\n  ctm -f club-cup.json standings"
)]
struct Cli {
    /// Tournament file to read and write.
    #[arg(short, long, global = true, value_name = "FILE", env = "CTM_FILE")]
    file: Option<PathBuf>,

    /// When to colourise output.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

    /// Print machine-readable JSON instead of a table.
    #[arg(long, global = true)]
    json: bool,

    /// Suppress informational messages.
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new tournament.
    New {
        /// Tournament name.
        #[arg(long, value_name = "NAME")]
        name: Option<String>,

        /// Read players from a file, one name[:rating] per line.
        #[arg(long, value_name = "FILE")]
        players: Option<PathBuf>,

        /// Play every pairing twice with reversed colours.
        #[arg(long)]
        double: bool,

        /// Where to write the tournament file (default: <name>.json).
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,

        /// Rating used when a player is entered without one.
        #[arg(long, value_name = "RATING", default_value_t = DEFAULT_RATING)]
        default_rating: u32,

        /// Overwrite the target file if it already exists.
        #[arg(long)]
        force: bool,
    },

    /// Pair the next round and record results interactively.
    Run,

    /// Add a player before the first round is paired.
    AddPlayer {
        /// Player as name[:rating].
        spec: String,
    },

    /// Set or correct a single result.
    #[command(name = "result")]
    SetResult {
        #[arg(long)]
        round: usize,
        #[arg(long)]
        board: usize,
        /// 1-0, 0-1, 1/2-1/2 (legacy: 1, 0, -1).
        result: String,
    },

    /// Print the standings.
    Standings {
        /// Comma-separated tiebreak chain, e.g. "buchholz,cut1,sb".
        #[arg(long, value_name = "CHAIN")]
        tiebreak: Option<String>,
    },

    /// List pairings and results.
    Rounds {
        #[arg(long)]
        round: Option<usize>,
    },

    /// Clear a result so it can be entered again.
    Undo {
        #[arg(long)]
        round: usize,
        #[arg(long)]
        board: usize,
    },

    /// Export games as PGN or the standings as CSV.
    Export {
        #[arg(value_enum)]
        format: ExportFormat,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    Pgn,
    Csv,
}

struct Ctx {
    painter: Painter,
    quiet: bool,
    json: bool,
    file: Option<PathBuf>,
}

pub fn run() -> ExitCode {
    let Cli {
        file,
        color,
        json,
        quiet,
        command,
    } = Cli::parse();

    let ctx = Ctx {
        painter: Painter::detect(color),
        quiet,
        json,
        file,
    };

    match dispatch(command, &ctx) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{} {}", ctx.painter.red("error:"), error);
            ExitCode::from(error.exit_code())
        }
    }
}

fn dispatch(command: Option<Command>, ctx: &Ctx) -> Result<()> {
    match command {
        None => wizard(ctx),
        Some(Command::New {
            name,
            players,
            double,
            out,
            default_rating,
            force,
        }) => cmd_new(name, players, double, out, default_rating, force, ctx),
        Some(Command::Run) => cmd_run(ctx),
        Some(Command::AddPlayer { spec }) => cmd_add_player(&spec, ctx),
        Some(Command::SetResult {
            round,
            board,
            result,
        }) => cmd_set_result(round, board, &result, ctx),
        Some(Command::Standings { tiebreak }) => cmd_standings(tiebreak.as_deref(), ctx),
        Some(Command::Rounds { round }) => cmd_rounds(round, ctx),
        Some(Command::Undo { round, board }) => cmd_undo(round, board, ctx),
        Some(Command::Export { format, output }) => cmd_export(format, output, ctx),
    }
}

fn require_file(ctx: &Ctx) -> Result<PathBuf> {
    ctx.file.clone().ok_or_else(|| {
        AppError::usage("no tournament file: pass -f <FILE> (or set CTM_FILE)")
    })
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
            pending_dash = false;
        } else if !out.is_empty() && !pending_dash {
            out.push('-');
            pending_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "tournament".to_string()
    } else {
        out
    }
}

fn default_path(name: &str) -> PathBuf {
    PathBuf::from(format!("{}.json", slug(name)))
}

fn cmd_new(
    name: Option<String>,
    players: Option<PathBuf>,
    double: bool,
    out: Option<PathBuf>,
    default_rating: u32,
    force: bool,
    ctx: &Ctx,
) -> Result<()> {
    let name = match name {
        Some(name) => name,
        None if io::stdin().is_terminal() => prompt::read_line("Name [Tournament]: ")?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Tournament".to_string()),
        None => "Tournament".to_string(),
    };

    let players = match players {
        Some(path) => parse_players_file(&path, default_rating)?,
        None => {
            if !io::stdin().is_terminal() {
                return Err(AppError::usage(
                    "no players given: pass --players <FILE> when stdin is not a terminal",
                ));
            }
            prompt_players(default_rating, ctx)?
        }
    };

    if players.len() < 2 {
        return Err(AppError::usage(format!(
            "a tournament needs at least 2 players (got {})",
            players.len()
        )));
    }

    let tournament = Tournament::new(name, players, double);
    let path = out
        .or_else(|| ctx.file.clone())
        .unwrap_or_else(|| default_path(&tournament.name));

    if path.exists() && !force {
        return Err(AppError::usage(format!(
            "{} already exists; pass --force to overwrite it",
            path.display()
        )));
    }

    storage::save(&tournament, &path)?;

    if !ctx.quiet {
        println!(
            "Created \"{}\" with {} players and {} rounds.",
            tournament.name,
            tournament.players.len(),
            tournament.total_rounds()
        );
        println!("  file: {}", path.display());
        println!("  next: ctm -f {} run", path.display());
    }

    Ok(())
}

fn parse_players_file(path: &Path, default_rating: u32) -> Result<Vec<Player>> {
    let raw = fs::read_to_string(path).map_err(|error| AppError::io(path, error))?;
    let mut players = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (index, line) in raw.lines().enumerate() {
        let number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, rating) =
            validate_spec(line, default_rating).map_err(|message| AppError::parse(path, number, message))?;

        if !seen.insert(name.to_lowercase()) {
            return Err(AppError::parse(
                path,
                number,
                format!("duplicate player name: {name}"),
            ));
        }

        players.push(Player::new(players.len(), name, rating));
    }

    Ok(players)
}

fn prompt_players(default_rating: u32, ctx: &Ctx) -> Result<Vec<Player>> {
    println!(
        "{}",
        ctx.painter
            .dim("Enter players as name[:rating]. Empty line finishes the list.")
    );

    let mut players: Vec<Player> = Vec::new();
    loop {
        let Some(line) = prompt::read_line("Player: ")? else {
            break;
        };
        if line.is_empty() {
            break;
        }

        let (name, rating) = match validate_spec(&line, default_rating) {
            Ok(parsed) => parsed,
            Err(message) => {
                println!("{} {}", ctx.painter.yellow("warning:"), message);
                continue;
            }
        };

        if players.iter().any(|player| player.name.eq_ignore_ascii_case(&name)) {
            println!(
                "{} a player named {name:?} already exists",
                ctx.painter.yellow("warning:")
            );
            continue;
        }

        println!("  {} ({rating})", ctx.painter.green(&name));
        players.push(Player::new(players.len(), name, rating));
    }

    Ok(players)
}

fn cmd_run(ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let mut tournament = storage::load(&path)?;

    if tournament.is_complete() {
        if !ctx.quiet {
            println!("This tournament is complete; showing the final standings.");
        }
        print_standings(&tournament, &Tiebreak::default_chain(), ctx);
        return Ok(());
    }

    play_loop(&mut tournament, &path, ctx)
}

fn play_loop(tournament: &mut Tournament, path: &Path, ctx: &Ctx) -> Result<()> {
    loop {
        let pending = tournament
            .rounds
            .iter()
            .position(|round| !round.is_complete());

        let index = match pending {
            Some(index) => index,
            None => {
                if tournament.rounds.len() >= tournament.total_rounds() {
                    break;
                }
                tournament.pair_next()?;
                let index = tournament.rounds.len() - 1;
                storage::save(tournament, path)?;
                index
            }
        };

        let round_number = tournament.rounds[index].number;
        print!(
            "{}",
            table::round_view(tournament, &tournament.rounds[index], &ctx.painter)
        );

        for board in tournament.rounds[index].pending_boards() {
            let (white, black) = {
                let game = tournament.rounds[index]
                    .game(board)
                    .ok_or_else(|| AppError::state(format!("board {board} vanished")))?;
                (game.white, game.black)
            };

            let prompt_text = format!(
                "  {} vs {} [1-0/0-1/1/2-1/2, q to save]: ",
                tournament.player_name(white),
                tournament.player_name(black)
            );

            match prompt::read_outcome(&prompt_text, &ctx.painter)? {
                Some(outcome) => {
                    tournament.set_result(round_number, board, outcome)?;
                    storage::save(tournament, path)?;
                    if !ctx.quiet {
                        println!("    recorded {}", outcome.pgn());
                    }
                }
                None => {
                    println!(
                        "Saved {}. Resume with `ctm -f {} run`.",
                        path.display(),
                        path.display()
                    );
                    return Ok(());
                }
            }
        }

        println!("\nRound {round_number} complete.\n");
        print_standings(tournament, &Tiebreak::default_chain(), ctx);
        println!();

        if tournament.is_complete() {
            break;
        }
    }

    println!("{}", ctx.painter.bold("Tournament complete."));
    Ok(())
}

fn cmd_add_player(spec: &str, ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let mut tournament = storage::load(&path)?;

    if !tournament.rounds.is_empty() {
        return Err(AppError::state(
            "cannot add players after the first round has been paired",
        ));
    }

    let (name, rating) =
        validate_spec(spec, DEFAULT_RATING).map_err(|message| AppError::usage(message))?;

    if tournament
        .players
        .iter()
        .any(|player| player.name.eq_ignore_ascii_case(&name))
    {
        return Err(AppError::usage(format!(
            "a player named {name:?} already exists"
        )));
    }

    tournament.players.push(Player::new(tournament.players.len(), name.clone(), rating));
    storage::save(&tournament, &path)?;

    if !ctx.quiet {
        println!(
            "Added {name} ({rating}). Now {} players, {} rounds.",
            tournament.players.len(),
            tournament.total_rounds()
        );
    }

    Ok(())
}

fn cmd_set_result(round: usize, board: usize, result: &str, ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let mut tournament = storage::load(&path)?;

    let outcome = Outcome::parse(result).ok_or_else(|| {
        AppError::usage(format!(
            "cannot read result {result:?}; use 1-0, 0-1 or 1/2-1/2 (legacy: 1, 0, -1)"
        ))
    })?;

    tournament.set_result(round, board, outcome)?;
    storage::save(&tournament, &path)?;

    if !ctx.quiet {
        println!("Round {round}, board {board}: {}.", outcome.pgn());
    }

    Ok(())
}

fn cmd_undo(round: usize, board: usize, ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let mut tournament = storage::load(&path)?;
    tournament.clear_result(round, board)?;
    storage::save(&tournament, &path)?;

    if !ctx.quiet {
        println!("Cleared round {round}, board {board}.");
    }

    Ok(())
}

fn cmd_standings(tiebreak: Option<&str>, ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let tournament = storage::load(&path)?;
    let chain = match tiebreak {
        Some(value) => Tiebreak::parse_chain(value)?,
        None => Tiebreak::default_chain(),
    };
    print_standings(&tournament, &chain, ctx);
    Ok(())
}

fn print_standings(tournament: &Tournament, chain: &[Tiebreak], ctx: &Ctx) {
    let rows = tournament.standings(chain);

    if ctx.json {
        match serde_json::to_string_pretty(&rows) {
            Ok(json) => println!("{json}"),
            Err(error) => eprintln!("error: could not serialise standings: {error}"),
        }
        return;
    }

    print!("{}", table::standings_table(tournament, &rows, &ctx.painter));
}

fn cmd_rounds(round: Option<usize>, ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let tournament = storage::load(&path)?;

    let rounds: Vec<&Round> = match round {
        Some(number) => {
            let stored = tournament.rounds.get(number.saturating_sub(1)).ok_or_else(|| {
                AppError::usage(format!("round {number} has not been paired yet"))
            })?;
            vec![stored]
        }
        None => tournament.rounds.iter().collect(),
    };

    if rounds.is_empty() {
        println!("No rounds have been paired yet.");
        return Ok(());
    }

    for stored in rounds {
        print!(
            "{}",
            table::round_view(&tournament, stored, &ctx.painter)
        );
    }

    Ok(())
}

fn cmd_export(format: ExportFormat, output: Option<PathBuf>, ctx: &Ctx) -> Result<()> {
    let path = require_file(ctx)?;
    let tournament = storage::load(&path)?;

    let content = match format {
        ExportFormat::Pgn => export::pgn(&tournament),
        ExportFormat::Csv => {
            let rows = tournament.standings(&Tiebreak::default_chain());
            export::standings_csv(&rows)
        }
    };

    match output {
        Some(output) => {
            fs::write(&output, content).map_err(|error| AppError::io(&output, error))?;
            if !ctx.quiet {
                println!("Wrote {}", output.display());
            }
        }
        None => print!("{content}"),
    }

    Ok(())
}

fn wizard(ctx: &Ctx) -> Result<()> {
    if !io::stdin().is_terminal() {
        return Err(AppError::usage(
            "no command given and stdin is not a terminal; run `ctm --help`",
        ));
    }

    if let Some(path) = &ctx.file {
        if path.exists() {
            let mut tournament = storage::load(path)?;
            println!(
                "Resuming \"{}\" from {}.",
                tournament.name,
                path.display()
            );
            return play_loop(&mut tournament, path, ctx);
        }
    }

    println!("{}", ctx.painter.bold("New round-robin tournament"));
    let name = prompt::read_line("Name [Tournament]: ")?
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Tournament".to_string());
    let players = prompt_players(DEFAULT_RATING, ctx)?;

    if players.len() < 2 {
        return Err(AppError::usage("a tournament needs at least 2 players"));
    }

    let tournament = Tournament::new(name, players, false);
    let path = ctx
        .file
        .clone()
        .unwrap_or_else(|| default_path(&tournament.name));
    storage::save(&tournament, &path)?;
    println!("Saved to {}\n", path.display());

    let mut tournament = tournament;
    play_loop(&mut tournament, &path, ctx)
}
