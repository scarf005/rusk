macro_rules! make_message {
    ($name:expr) => { format!("hello {}", $name) };
}

pub fn make_message(name: &str) -> String {
    make_message!(name)
}

pub fn normalized_names(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_ascii_lowercase()) }
        })
        .collect::<Vec<String>>()
}
