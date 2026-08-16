use unicode_width::UnicodeWidthChar;

pub fn column_widths<'a, const N: usize>(rows: impl IntoIterator<Item = [&'a str; N]>) -> [usize; N] {
    let mut widths = [0; N];
    for row in rows {
        for (index, cell) in row.into_iter().enumerate() {
            widths[index] = widths[index].max(display_width(cell));
        }
    }
    widths
}

pub fn pad_cell(value: &str, width: usize) -> String {
    format!("{value}{}", " ".repeat(width.saturating_sub(display_width(value))))
}

pub fn display_width(value: &str) -> usize {
    value.chars().map(|character| UnicodeWidthChar::width(character).unwrap_or(0)).sum()
}

pub fn truncate_cell(value: &str, width: usize) -> String {
    if display_width(value) <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_string();
    }

    let mut truncated = String::new();
    let mut current_width = 0;
    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if current_width + character_width + 1 > width {
            break;
        }
        truncated.push(character);
        current_width += character_width;
    }
    truncated.push('…');
    truncated
}

pub fn fit_cell(value: &str, width: usize) -> String {
    pad_cell(&truncate_cell(value, width), width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_preserves_utf8_and_marks_the_boundary() {
        assert_eq!(truncate_cell("café🙂", 5), "café…");
        assert_eq!(truncate_cell("café🙂", 0), "");
        assert_eq!(truncate_cell("café🙂", 1), "…");
        assert_eq!(display_width("café🙂"), 6);
    }

    #[test]
    fn padding_and_column_widths_use_display_width() {
        assert_eq!(pad_cell("🙂", 3), "🙂 ");
        assert_eq!(column_widths([["a", "🙂"], ["long", "x"]]), [4, 2]);
    }
}
