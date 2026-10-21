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
    Easter,
    Valentine,
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
            Mode::Auto => Ok(Some(if in_easter_window(year, month, day) {
                Ambiance::Easter
            } else if matches!((month, day), (1, 31) | (2, 1..=14)) {
                Ambiance::Valentine
            } else {
                match (month, day) {
                    (10, _) => Ambiance::Halloween,
                    (11, 25..=30) | (12, 1..=25) => Ambiance::Christmas,
                    (3..=5, _) => Ambiance::Spring,
                    (6..=8, _) => Ambiance::Summer,
                    (9..=11, _) => Ambiance::Autumn,
                    _ => Ambiance::Winter,
                }
            })),
        }
    }
}

fn in_easter_window(year: i32, month: u8, day: u8) -> bool {
    // Meeus/Jones/Butcher Gregorian computus. Compare ordinal civil dates, not
    // elapsed seconds, so the device's local DST transition cannot shift it.
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let easter_month = (h + l - 7 * m + 114) / 31;
    let easter_day = (h + l - 7 * m + 114) % 31 + 1;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let ordinal = |month: i32, day: i32| {
        let month_days = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        month_days[..(month - 1) as usize].iter().sum::<i32>() + day
    };
    let point = ordinal(i32::from(month), i32::from(day));
    let end = ordinal(easter_month, easter_day);
    point >= end - 30 && point <= end
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_calendar_boundaries_and_manual_off_are_local() {
        let mut a = Appearance::default();
        for (month, day, expected) in [
            (1, 30, Ambiance::Winter),
            (1, 31, Ambiance::Valentine),
            (2, 14, Ambiance::Valentine),
            (2, 15, Ambiance::Winter),
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

    #[test]
    fn western_easter_window_is_annual_and_returns_to_spring() {
        for (year, start, end, next) in [
            (2026, (3, 6), (4, 5), (4, 6)),
            (2027, (2, 26), (3, 28), (3, 29)),
            (2038, (3, 26), (4, 25), (4, 26)),
        ] {
            let appearance = Appearance::default();
            assert_eq!(
                appearance.resolve(year, start.0, start.1).unwrap(),
                Some(Ambiance::Easter)
            );
            assert_eq!(
                appearance.resolve(year, end.0, end.1).unwrap(),
                Some(Ambiance::Easter)
            );
            assert_eq!(
                appearance.resolve(year, next.0, next.1).unwrap(),
                Some(Ambiance::Spring)
            );
        }
    }
}
