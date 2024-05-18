/// Convert a path to the smallest possible representation
/// This is done by handling shortcuts like `..` and `.` in the path
pub fn normalize_path(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    let mut new_parts = Vec::new();
    for part in parts.iter() {
        if part == &"." {
            continue;
        } else if part == &".." {
            new_parts.pop();
        } else {
            new_parts.push(part);
        }
    }
    // Remove double slashes
    let new_parts = new_parts
        .iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    let new_parts = new_parts.iter().map(|p| p.to_string()).collect::<Vec<_>>();
    let new_parts = new_parts.join("/");
    new_parts
}

/// Split the final file/dir -name of the path
pub fn split_name_path(path: &str) -> (String, String) {
    let mut dir_parts = path.split("/").collect::<Vec<_>>();
    let name_part = dir_parts.split_off(dir_parts.len() - 1);
    let name_part = name_part.first().unwrap();
    (dir_parts.join("/"), name_part.to_string())
}
