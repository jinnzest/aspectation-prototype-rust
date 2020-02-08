extern crate aspectation_prototype;

use std::collections::HashMap;

use aspectation_prototype::semantic::hashes::{read_hashes, write_hashes};
use aspectation_prototype::semantic::model::FuncName;
use aspectation_prototype::semantic::model::SemanticHash;
use aspectation_prototype::utils::{create_all_paths, remove_all_paths};
use aspectation_prototype::SETTINGS;
use std::error::Error;

fn set_path() -> Result<(), Box<dyn Error>> {
    SETTINGS
        .write()?
        .set("project_path", "tests_output/hashes")?;
    Ok(())
}

fn setup() {
    match set_path() {
        Err(_) => panic!(),
        _ => {}
    }
    remove_all_paths();
    create_all_paths();
}

#[test]
fn read_equals_to_written() {
    setup();
    let mut initial_hashes = HashMap::new();
    initial_hashes.insert(
        FuncName::new("test_func"),
        SemanticHash::new("test_func_hash"),
    );
    write_hashes("test", &initial_hashes);
    let read_hashes = read_hashes("test");
    let read_hashes_converted: HashMap<FuncName, SemanticHash> = read_hashes
        .unwrap()
        .iter()
        .map(|(s, n)| (n.clone(), s.clone()))
        .collect();
    assert_eq!(read_hashes_converted, initial_hashes);
}
