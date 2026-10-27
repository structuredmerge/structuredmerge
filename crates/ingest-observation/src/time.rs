//! RFC3339 UTC timestamp validation.

/// Validates an RFC3339 UTC timestamp with strict lexical requirements.
///
/// Requirements:
/// - Must end with 'Z'
/// - Date portion: YYYY-MM-DD with exact separators
/// - Time portion: HH:MM:SS with exact separators
/// - Optional fractional seconds: .NNN
/// - Calendar-valid date/time (delegated to chrono)
pub fn is_valid_rfc3339_utc(value: &str) -> bool {
    if !value.ends_with('Z') {
        return false;
    }
    let datetime = &value[..value.len() - 1];
    let (date, time) = match datetime.split_once('T') {
        Some(pair) => pair,
        None => return false,
    };
    if date.len() != 10 || time.is_empty() {
        return false;
    }
    // Validate date separators and digits
    if date.chars().nth(4) != Some('-') || date.chars().nth(7) != Some('-') {
        return false;
    }
    if !date.chars().enumerate().all(|(i, c)| {
        if i == 4 || i == 7 {
            return c == '-';
        }
        c.is_ascii_digit()
    }) {
        return false;
    }
    // Validate time separators
    let time = time.strip_suffix('Z').unwrap_or(time);
    let (time, _fraction) = time.split_once('.').unwrap_or((time, ""));
    if time.len() != 8 || time.chars().nth(2) != Some(':') || time.chars().nth(5) != Some(':') {
        return false;
    }
    if !time.chars().enumerate().all(|(i, c)| {
        if i == 2 || i == 5 {
            return c == ':';
        }
        c.is_ascii_digit()
    }) {
        return false;
    }
    // Delegate calendar validation to chrono
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}
