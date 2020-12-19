pub fn read_file_whole(filepath: &str) -> Result<Vec<u8>, String> {
    todo!()
    // file_fetcher::http::open_bytes_str(filepath)
    //     .map_err(|error| format!("Could not fetch file '{}' : {}", filepath, error))
}

pub fn path_join(first: &str, second: &str) -> String {
    if first.ends_with('/') || first.ends_with('\\') {
        format!("{}{}", first, second).replace("\\", "/")
    } else {
        format!("{}/{}", first, second).replace("\\", "/")
    }
}
