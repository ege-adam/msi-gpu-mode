use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Hybrid,
    Discrete,
    Integrated,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Hybrid, Mode::Discrete, Mode::Integrated];

    pub(crate) fn code(self) -> u8 {
        match self {
            Mode::Hybrid => 0,
            Mode::Discrete => 1,
            Mode::Integrated => 2,
        }
    }

    pub(crate) fn from_code(code: u8) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.code() == code)
    }

    pub fn id(self) -> &'static str {
        match self {
            Mode::Hybrid => "hybrid",
            Mode::Discrete => "dgpu",
            Mode::Integrated => "igpu",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Mode::Hybrid => "Hybrid",
            Mode::Discrete => "Discrete only",
            Mode::Integrated => "Integrated only",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Mode::Hybrid => "Integrated GPU drives the display, discrete on demand",
            Mode::Discrete => "Display wired to the discrete GPU, integrated GPU off",
            Mode::Integrated => "Discrete GPU powered off, longest battery life",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "hybrid" => Ok(Mode::Hybrid),
            "dgpu" | "discrete" => Ok(Mode::Discrete),
            "igpu" | "integrated" => Ok(Mode::Integrated),
            other => Err(format!("unknown mode '{other}', expected hybrid, dgpu or igpu")),
        }
    }
}
