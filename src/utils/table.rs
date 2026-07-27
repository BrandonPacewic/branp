pub fn column_widths<'a, const N: usize>(rows: impl IntoIterator<Item = [&'a str; N]>) -> [usize; N] {
    let mut widths = [0; N];
    for row in rows {
        for (index, cell) in row.into_iter().enumerate() {
            widths[index] = widths[index].max(cell.len());
        }
    }
    widths
}

pub fn pad_cell(value: &str, width: usize) -> String {
    format!("{value:<width$}")
}
