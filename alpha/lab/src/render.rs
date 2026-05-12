pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if headers.is_empty() {
        return;
    }
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<_>>();
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            if idx < widths.len() {
                widths[idx] = widths[idx].max(cell.len());
            }
        }
    }

    println!("{}", format_row(headers, &widths));
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>();
    println!(
        "{}",
        format_row(
            &separator.iter().map(String::as_str).collect::<Vec<_>>(),
            &widths
        )
    );
    for row in rows {
        let cells = row.iter().map(String::as_str).collect::<Vec<_>>();
        println!("{}", format_row(&cells, &widths));
    }
}

fn format_row(cells: &[&str], widths: &[usize]) -> String {
    cells
        .iter()
        .enumerate()
        .map(|(idx, cell)| format!("{cell:width$}", width = widths[idx]))
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn fmt_opt(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "-".to_string())
}

pub fn fmt_bool(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
