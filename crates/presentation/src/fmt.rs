//! Formatage des nombres et des durées, à la française.

const SHORT_SUFFIXES: [&str; 11] = [
    "k", "M", "Md", "Bn", "Bd", "Tn", "Td", "Qa", "Qad", "Qi", "Qid",
];

const LONG_SCALE: [&str; 10] = [
    "millions",
    "milliards",
    "billions",
    "billiards",
    "trillions",
    "trilliards",
    "quadrillions",
    "quadrilliards",
    "quintillions",
    "quintilliards",
];

fn trim_zero(s: String) -> String {
    match s.strip_suffix(".0") {
        Some(t) => t.to_string(),
        None => s,
    }
}

fn three_digits(v: f64) -> String {
    if v < 10.0 {
        format!("{v:.2}")
    } else if v < 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.0}")
    }
}

/// Format court : `7.5`, `342`, `1.23k`, `45.6M`, `789Md`, `1.23e36`.
pub fn short(n: f64) -> String {
    if !n.is_finite() {
        return "∞".into();
    }
    if n < 0.0 {
        return format!("-{}", short(-n));
    }
    if n < 10.0 {
        let s = trim_zero(format!("{n:.1}"));
        return if s == "10" { "10".into() } else { s };
    }
    if n < 999.5 {
        return format!("{n:.0}");
    }
    let mut exp3 = (n.log10() / 3.0).floor() as i32;
    let mut v = n / 10f64.powi(exp3 * 3);
    if v < 1.0 {
        exp3 -= 1;
        v *= 1000.0;
    }
    // Arrondi qui déborde (999.96k → 1.00M).
    if v >= 999.5 {
        exp3 += 1;
        v /= 1000.0;
    }
    let idx = exp3 - 1;
    if idx < 0 {
        return format!("{n:.0}");
    }
    match SHORT_SUFFIXES.get(idx as usize) {
        Some(suffix) => format!("{}{}", three_digits(v), suffix),
        None => format!("{n:.2e}"),
    }
}

/// Entier avec espace comme séparateur de milliers : `123 456`.
pub fn grouped(n: f64) -> String {
    let n = n.max(0.0).floor() as u64;
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// Format du gros compteur : `(chiffres, mot)`.
/// `< 1e6` → `("123 456", "poussières d'étoile")`, sinon `("1.234", "milliards")`.
pub fn big(n: f64) -> (String, String) {
    let n = if n.is_finite() { n.max(0.0) } else { 0.0 };
    if n < 1e6 {
        let word = if n < 2.0 {
            "poussière d'étoile"
        } else {
            "poussières d'étoile"
        };
        return (grouped(n), word.into());
    }
    let mut exp3 = (n.log10() / 3.0).floor() as i32;
    let mut mant = n / 10f64.powi(exp3 * 3);
    if (mant * 1000.0).round() / 1000.0 >= 1000.0 {
        exp3 += 1;
        mant /= 1000.0;
    }
    let idx = exp3 - 2;
    match LONG_SCALE.get(idx.max(0) as usize) {
        Some(word) if idx >= 0 => (format!("{mant:.3}"), (*word).into()),
        _ => {
            let e = n.log10().floor() as i32;
            let m = n / 10f64.powi(e);
            (format!("{m:.3}"), format!("× 10^{e}"))
        }
    }
}

/// Durée lisible : `1h 02m`, `3m 20s`, `45s`.
pub fn duration(secs: f64) -> String {
    let s = if secs.is_finite() {
        secs.max(0.0) as u64
    } else {
        0
    };
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

/// Horodatage `mm:ss` (les minutes peuvent dépasser 59).
pub fn clock(secs: f64) -> String {
    let s = if secs.is_finite() {
        secs.max(0.0) as u64
    } else {
        0
    };
    if s >= 3600 {
        // Au-delà d'une heure : heures et minutes (« 111h06 »).
        format!("{}h{:02}", s / 3600, s % 3600 / 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

/// Compte à rebours `m:ss`.
pub fn countdown(secs: f64) -> String {
    let s = if secs.is_finite() {
        secs.max(0.0).ceil() as u64
    } else {
        0
    };
    format!("{}:{:02}", s / 60, s % 60)
}

/// Pourcentage entier : `0.6 → "60%"`.
pub fn percent(fraction: f64) -> String {
    format!("{}%", (fraction * 100.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_small_numbers() {
        assert_eq!(short(0.0), "0");
        assert_eq!(short(1.0), "1");
        assert_eq!(short(2.5), "2.5");
        assert_eq!(short(0.1), "0.1");
        assert_eq!(short(12.0), "12");
        assert_eq!(short(999.0), "999");
    }

    #[test]
    fn short_suffixes() {
        assert_eq!(short(1_234.0), "1.23k");
        assert_eq!(short(45_600_000.0), "45.6M");
        assert_eq!(short(789e9), "789Md");
        assert_eq!(short(1.5e12), "1.50Bn");
        assert_eq!(short(2e15), "2.00Bd");
        assert_eq!(short(3e33), "3.00Qid");
        assert_eq!(short(1.234e36), "1.23e36");
        assert_eq!(short(999_960.0), "1.00M");
        assert_eq!(short(-1500.0), "-1.50k");
    }

    #[test]
    fn big_format() {
        assert_eq!(
            big(123_456.7),
            ("123 456".to_string(), "poussières d'étoile".to_string())
        );
        assert_eq!(big(1.0).1, "poussière d'étoile");
        assert_eq!(big(0.0).0, "0");
        assert_eq!(
            big(1_234_000.0),
            ("1.234".to_string(), "millions".to_string())
        );
        assert_eq!(big(5.5e9), ("5.500".to_string(), "milliards".to_string()));
        assert_eq!(big(42e12).1, "billions");
        assert_eq!(big(9.9e35).1, "quintilliards");
        assert_eq!(big(2e40), ("2.000".to_string(), "× 10^40".to_string()));
        assert_eq!(big(999_999_999.9).1, "milliards");
    }

    #[test]
    fn grouping() {
        assert_eq!(grouped(0.0), "0");
        assert_eq!(grouped(999.0), "999");
        assert_eq!(grouped(1000.0), "1 000");
        assert_eq!(grouped(1_234_567.0), "1 234 567");
    }

    #[test]
    fn durations() {
        assert_eq!(duration(45.0), "45s");
        assert_eq!(duration(200.0), "3m 20s");
        assert_eq!(duration(3720.0), "1h 02m");
        assert_eq!(duration(8040.0), "2h 14m");
        assert_eq!(clock(754.0), "12:34");
        assert_eq!(clock(400_000.0), "111h06");
        assert_eq!(countdown(24.2), "0:25");
        assert_eq!(percent(0.6), "60%");
    }
}
