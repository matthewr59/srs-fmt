use std::fmt;

/// A single scheduling row in canonical form.
#[derive(Debug, Clone)]
pub struct Card {
    pub front: String,
    pub back: String,
    pub interval_days: u32,
    /// Ease factor times 100, e.g. 250 means 2.50.
    pub ease: u32,
    /// "YYYY-MM-DD", or empty for a card that has never been scheduled.
    pub due_date: String,
}

#[derive(Debug)]
pub enum ParseError {
    EmptyField(&'static str),
    WrongFieldCount(usize),
    Whitespace(&'static str),
    BadInterval(String),
    BadEase(String),
    BadDate(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyField(name) => write!(f, "field '{name}' is empty"),
            ParseError::WrongFieldCount(n) => write!(f, "expected 5 fields, found {n}"),
            ParseError::Whitespace(name) => {
                write!(f, "field '{name}' has leading or trailing whitespace")
            }
            ParseError::BadInterval(s) => write!(f, "interval '{s}' is not a non-negative integer"),
            ParseError::BadEase(s) => write!(f, "ease '{s}' is not a valid ease factor"),
            ParseError::BadDate(s) => write!(f, "date '{s}' is not a valid date"),
        }
    }
}

// Tried in order when guessing how a lenient row is separated.
const CANDIDATE_DELIMITERS: [char; 4] = ['\t', ';', ',', '|'];

fn detect_delimiter(line: &str) -> Option<char> {
    CANDIDATE_DELIMITERS
        .into_iter()
        .find(|&d| line.split(d).count() >= 5)
}

/// Parses one input row into a `Card`. Strict mode requires the canonical
/// tab-separated, whitespace-clean, ISO-date shape exactly; lenient mode
/// guesses a delimiter, trims fields, and accepts a few common date and
/// decimal variants seen in real exports.
pub fn parse_line(line: &str, lenient: bool) -> Result<Card, ParseError> {
    let delimiter = if lenient {
        detect_delimiter(line).unwrap_or('\t')
    } else {
        '\t'
    };

    let mut fields: Vec<&str> = line.split(delimiter).collect();

    if lenient {
        // Some exporters always terminate rows with the delimiter, which
        // leaves a trailing empty field.
        while fields.len() > 5 && fields.last().is_some_and(|s| s.trim().is_empty()) {
            fields.pop();
        }
    }

    if fields.len() != 5 {
        return Err(ParseError::WrongFieldCount(fields.len()));
    }

    let front = clean_field(fields[0], "front", lenient)?;
    let back = clean_field(fields[1], "back", lenient)?;
    let interval_raw = clean_field(fields[2], "interval", lenient)?;
    let ease_raw = clean_field(fields[3], "ease", lenient)?;
    let date_raw = clean_field(fields[4], "due_date", lenient)?;

    if front.is_empty() {
        return Err(ParseError::EmptyField("front"));
    }
    if back.is_empty() {
        return Err(ParseError::EmptyField("back"));
    }

    Ok(Card {
        front,
        back,
        interval_days: parse_interval(&interval_raw, lenient)?,
        ease: parse_ease(&ease_raw, lenient)?,
        due_date: parse_due_date(&date_raw, lenient)?,
    })
}

fn clean_field(raw: &str, name: &'static str, lenient: bool) -> Result<String, ParseError> {
    if lenient {
        Ok(raw.trim().to_string())
    } else if raw != raw.trim() {
        Err(ParseError::Whitespace(name))
    } else {
        Ok(raw.to_string())
    }
}

fn parse_interval(raw: &str, lenient: bool) -> Result<u32, ParseError> {
    if !lenient {
        return raw
            .parse::<u32>()
            .map_err(|_| ParseError::BadInterval(raw.to_string()));
    }
    // Spreadsheet exports often turn whole numbers into "3.0".
    let trimmed = raw.strip_suffix(".0").unwrap_or(raw);
    trimmed
        .parse::<u32>()
        .map_err(|_| ParseError::BadInterval(raw.to_string()))
}

fn parse_ease(raw: &str, lenient: bool) -> Result<u32, ParseError> {
    if !lenient {
        return raw.parse::<u32>().map_err(|_| ParseError::BadEase(raw.to_string()));
    }
    if let Ok(v) = raw.parse::<u32>() {
        return Ok(v);
    }
    // Lenient mode accepts a decimal ease factor, comma or dot, e.g.
    // "2.5" or "2,5" both mean the same thing as canonical "250".
    let normalized = raw.replace(',', ".");
    let (whole, frac) = normalized
        .split_once('.')
        .ok_or_else(|| ParseError::BadEase(raw.to_string()))?;
    let whole: u32 = whole.parse().map_err(|_| ParseError::BadEase(raw.to_string()))?;
    let frac_digits = format!("{frac:0<2}");
    let frac: u32 = frac_digits[..2]
        .parse()
        .map_err(|_| ParseError::BadEase(raw.to_string()))?;
    Ok(whole * 100 + frac)
}

fn parse_due_date(raw: &str, lenient: bool) -> Result<String, ParseError> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    if let Some(d) = try_iso_date(raw) {
        return Ok(d);
    }
    if !lenient {
        return Err(ParseError::BadDate(raw.to_string()));
    }
    for sep in ['/', '.'] {
        let parts: Vec<&str> = raw.split(sep).collect();
        if parts.len() != 3 {
            continue;
        }
        // Ambiguous slash/dot dates are assumed day-first, then tried
        // year-first as a fallback for exporters that write YYYY/MM/DD.
        if let Some(d) = build_date(parts[2], parts[1], parts[0]) {
            return Ok(d);
        }
        if let Some(d) = build_date(parts[0], parts[1], parts[2]) {
            return Ok(d);
        }
    }
    Err(ParseError::BadDate(raw.to_string()))
}

fn try_iso_date(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 {
        return None;
    }
    build_date(parts[0], parts[1], parts[2])
}

fn build_date(y: &str, m: &str, d: &str) -> Option<String> {
    let year: u32 = y.parse().ok()?;
    let month: u32 = m.parse().ok()?;
    let day: u32 = d.parse().ok()?;
    if !(1970..=9999).contains(&year) || month == 0 || month > 12 {
        return None;
    }
    if day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

impl Card {
    pub fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.front, self.back, self.interval_days, self.ease, self.due_date
        )
    }
}
