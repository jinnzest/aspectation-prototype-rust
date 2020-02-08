extern crate aspectation_prototype;

use std::collections::HashMap;

use aspectation_prototype::aspects::hints::{read_all_hints, write_all_hints};
use aspectation_prototype::aspects::register::{AspectWrapper, HintWrapper};
use aspectation_prototype::aspects::side_effect::model::{SideEffectAspect, SideEffectHint};
use aspectation_prototype::semantic::model::FuncName;
use aspectation_prototype::utils::{create_all_paths, remove_all_paths};
use aspectation_prototype::SETTINGS;
use std::error::Error;
use std::rc::Rc;

fn set_path() -> Result<(), Box<dyn Error>> {
    SETTINGS
        .write()?
        .set("project_path", "tests_output/hints")?;
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
    let aspects = vec![AspectWrapper::SideEffect(Rc::new(SideEffectAspect {}))];
    let mut hints = HashMap::new();
    let name = FuncName::new("test_func");
    hints.insert(
        name.clone(),
        vec![HintWrapper::SideEffect(Rc::new(
            SideEffectHint::AnySideEffect,
        ))],
    );
    write_all_hints(&hints, &aspects);
    let read_hints = read_all_hints(&aspects);
    assert_eq!(
        hints.get(&name).unwrap(),
        read_hints.unwrap().get(&name).unwrap()
    );
}
