use SETTINGS;

pub fn project_path() -> String {
    SETTINGS
        .read()
        .unwrap()
        .get::<String>("project_path")
        .unwrap()
}

pub fn src_path() -> String {
    format!("{}/src", project_path())
}

pub fn src_file(file_name: &str) -> String {
    format!("{}/{}.astn", src_path(), file_name)
}

pub fn hints_path() -> String {
    format!("{}/hints", project_path())
}

pub fn hints_file_path(file_name: &str) -> String {
    format!("{}/{}.hnt", hints_path(), file_name)
}

pub fn analytics_path() -> String {
    format!("{}/analytics", project_path())
}

pub fn analytics_file_path(file_name: &str) -> String {
    format!("{}/{}.altc", analytics_path(), file_name)
}

pub fn hashes_path() -> String {
    format!("{}/hashes", project_path())
}

pub fn hashes_file(file_name: &str) -> String {
    format!("{}/{}.hsh", hashes_path(), file_name)
}
