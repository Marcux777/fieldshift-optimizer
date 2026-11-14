// AI-assisted (OpenBot agent)
//! Gregorian civil dates. Python ordinal: 0001-01-01 is 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}
pub fn is_leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, &'static str> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err("invalid civil date");
        }
        let lengths = [
            31,
            if is_leap(year) { 29 } else { 28 },
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
        if day == 0 || day > lengths[month as usize - 1] {
            return Err("invalid civil date");
        }
        Ok(Self { year, month, day })
    }
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        let compact = s.replace('-', "");
        if compact.len() != 8 || !compact.bytes().all(|b| b.is_ascii_digit()) {
            return Err("invalid date format");
        }
        Self::new(
            compact[..4].parse().map_err(|_| "year")?,
            compact[4..6].parse().map_err(|_| "month")?,
            compact[6..].parse().map_err(|_| "day")?,
        )
    }
    pub fn ordinal(self) -> i32 {
        let y = self.year - 1;
        let mut n = 365 * y + y / 4 - y / 100 + y / 400;
        for m in 1..self.month {
            n += match m {
                2 => {
                    if is_leap(self.year) {
                        29
                    } else {
                        28
                    }
                }
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
        }
        n + i32::from(self.day)
    }
    pub fn agricultural_year(self) -> i32 {
        self.year - i32::from(self.month < 9)
    }
    pub fn agricultural_day(self) -> Option<usize> {
        if self.month == 2 && self.day == 29 {
            return None;
        }
        let start = Self {
            year: self.agricultural_year(),
            month: 9,
            day: 1,
        };
        let mut n = self.ordinal() - start.ordinal();
        if self.month >= 3 && self.month < 9 && is_leap(self.year) {
            n -= 1;
        }
        Some(n as usize)
    }
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
