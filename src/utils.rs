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
