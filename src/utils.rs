pub async fn extract_vec<T, E1, E2>(res: Result<Result<Vec<T>, E1>, E2>) -> Vec<T>
where
    T: std::fmt::Debug,
    E1: std::fmt::Display,
    E2: std::fmt::Display,
{
    match res {
        Ok(inner_result) => match inner_result {
            Ok(items) => items,
            Err(e) => {
                eprintln!("Error fetching data: {}", e);
                Vec::new()
            }
        },
        Err(e) => {
            eprintln!("Task panicked: {}", e);
            Vec::new()
        }
    }
}

pub fn sanitize_slug(input: &str) -> String {
    // trim and lowercase input
    let s = input.trim().to_lowercase();
    let mut out = String::with_capacity(s.len());

    // Turn non alphanumerics to - and drop brackets
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if c.is_ascii_whitespace() || c == '-' || c == '_' {
            out.push('-');
        }
    }

    // Collapse back to back dashes
    let slug = out
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    slug.trim_matches('-').to_string()
}
