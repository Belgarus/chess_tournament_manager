use crate::error::{AppError, Result};

/// Tiebreak criteria, applied in the order the user asks for them.
///
/// Points always come first and are not part of this enum; the chain only
/// decides between players on the same score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tiebreak {
    Buchholz,
    BuchholzCut1,
    SonnebornBerger,
    DirectEncounter,
    Wins,
}

impl Tiebreak {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "buchholz" | "bh" => Some(Tiebreak::Buchholz),
            "cut1" | "cut-1" | "buchholz-cut1" | "buchholz-cut-1" => Some(Tiebreak::BuchholzCut1),
            "sb" | "sonneborn" | "sonneborn-berger" => Some(Tiebreak::SonnebornBerger),
            "de" | "direct" | "direct-encounter" => Some(Tiebreak::DirectEncounter),
            "wins" | "most-wins" => Some(Tiebreak::Wins),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Tiebreak::Buchholz => "buchholz",
            Tiebreak::BuchholzCut1 => "cut1",
            Tiebreak::SonnebornBerger => "sb",
            Tiebreak::DirectEncounter => "de",
            Tiebreak::Wins => "wins",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tiebreak::Buchholz => "Buchholz",
            Tiebreak::BuchholzCut1 => "Buchholz cut-1",
            Tiebreak::SonnebornBerger => "Sonneborn-Berger",
            Tiebreak::DirectEncounter => "direct encounter",
            Tiebreak::Wins => "most wins",
        }
    }

    /// Buchholz first, matching what the old README promised, then progressively
    /// finer-grained criteria.
    pub fn default_chain() -> Vec<Tiebreak> {
        vec![
            Tiebreak::Buchholz,
            Tiebreak::BuchholzCut1,
            Tiebreak::SonnebornBerger,
            Tiebreak::DirectEncounter,
            Tiebreak::Wins,
        ]
    }

    pub fn parse_chain(input: &str) -> Result<Vec<Tiebreak>> {
        let mut chain = Vec::new();
        for part in input.split([',', ' ', ';']).filter(|p| !p.trim().is_empty()) {
            let tiebreak = Tiebreak::parse(part).ok_or_else(|| {
                AppError::usage(format!(
                    "unknown tiebreak {part:?}; use buchholz, cut1, sb, de or wins"
                ))
            })?;
            if !chain.contains(&tiebreak) {
                chain.push(tiebreak);
            }
        }
        if chain.is_empty() {
            return Err(AppError::usage("the tiebreak list is empty"));
        }
        Ok(chain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chain_in_order() {
        let chain = Tiebreak::parse_chain("sb, buchholz, wins").unwrap();
        assert_eq!(
            chain,
            vec![
                Tiebreak::SonnebornBerger,
                Tiebreak::Buchholz,
                Tiebreak::Wins
            ]
        );
    }

    #[test]
    fn rejects_unknown_criterion() {
        let err = Tiebreak::parse_chain("buchholz, elo").unwrap_err();
        assert!(err.to_string().contains("elo"), "{err}");
    }

    #[test]
    fn default_chain_starts_with_buchholz() {
        assert_eq!(Tiebreak::default_chain()[0], Tiebreak::Buchholz);
    }
}
