#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub rating: u32,
    pub whites: u32,
    pub blacks: u32,
    pub last_color: Option<Color>,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub buchholz: f32,
}

impl Player {
    pub fn new(name: String, rating: u32) -> Self {
        Self {
            name,
            rating,
            whites: 0,
            blacks: 0,
            last_color: None,
            wins: 0,
            draws: 0,
            losses: 0,
            buchholz: 0.0,
        }
    }
    
    pub fn points(&self) -> f32 {
        self.wins as f32 + self.draws as f32 * 0.5
    }
}
