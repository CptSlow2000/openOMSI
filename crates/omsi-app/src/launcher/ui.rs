//! What the launcher's pages share about texts and dates: whether a text matches what is
//! typed into a search (in English or as translated), and a date.

pub fn matches(text: &str, q: &str) -> bool {
    text.to_lowercase().contains(q) || omsi_ui::tr(text).to_lowercase().contains(q)
}

pub const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn parse_date(s: &str) -> (i32, u32, u32) {
    let mut it = s.trim().split('-');
    let y = it.next().and_then(|x| x.parse().ok()).unwrap_or(1989);
    let m = it.next().and_then(|x| x.parse().ok()).unwrap_or(5).clamp(1, 12);
    let d = it.next().and_then(|x| x.parse().ok()).unwrap_or(30).clamp(1, 31);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_is_read() {
        assert_eq!(parse_date("1989-05-30"), (1989, 5, 30));
    }
}
