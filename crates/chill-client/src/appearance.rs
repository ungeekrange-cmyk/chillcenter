//! Device-local, deterministic calendar. Never reads or writes a profile.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Auto,
    Manual,
    Off,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ambiance {
    Spring,
    Summer,
    #[default]
    Autumn,
    Winter,
    Christmas,
    Halloween,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub mode: Mode,
    pub manual: Ambiance,
}
impl Appearance {
    pub fn resolve(&self, year: i32, month: u8, day: u8) -> Result<Option<Ambiance>, String> {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days = match month {
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        };
        if !(1..=9999).contains(&year) || day == 0 || day > days {
            return Err("Date locale invalide.".into());
        }
        match self.mode {
            Mode::Off => Ok(None),
            Mode::Manual => Ok(Some(self.manual)),
            Mode::Auto => Ok(Some(match (month, day) {
                (10, _) => Ambiance::Halloween,
                (11, 25..=30) | (12, 1..=25) => Ambiance::Christmas,
                (3..=5, _) => Ambiance::Spring,
                (6..=8, _) => Ambiance::Summer,
                (9..=11, _) => Ambiance::Autumn,
                _ => Ambiance::Winter,
            })),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_calendar_boundaries_and_manual_off_are_local() {
        let mut a = Appearance::default();
        for (month, day, expected) in [
            (2, 28, Ambiance::Winter),
            (3, 1, Ambiance::Spring),
            (5, 31, Ambiance::Spring),
            (6, 1, Ambiance::Summer),
            (8, 31, Ambiance::Summer),
            (9, 1, Ambiance::Autumn),
            (9, 30, Ambiance::Autumn),
            (10, 1, Ambiance::Halloween),
            (10, 31, Ambiance::Halloween),
            (11, 1, Ambiance::Autumn),
            (11, 24, Ambiance::Autumn),
            (11, 25, Ambiance::Christmas),
            (12, 25, Ambiance::Christmas),
            (12, 26, Ambiance::Winter),
        ] {
            assert_eq!(a.resolve(2026, month, day).unwrap(), Some(expected));
        }
        assert!(a.resolve(2026, 2, 29).is_err());
        assert!(a.resolve(2028, 2, 29).is_ok());
        a.mode = Mode::Manual;
        a.manual = Ambiance::Summer;
        assert_eq!(a.resolve(2026, 12, 25).unwrap(), Some(Ambiance::Summer));
        a.mode = Mode::Off;
        assert_eq!(a.resolve(2026, 10, 31).unwrap(), None);
    }
}
