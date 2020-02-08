extern crate aspectation_prototype;

use std::collections::{HashMap, HashSet};

use aspectation_prototype::aspects::analytics::{read_analytics_for_aspects, write_all_analytics};
use aspectation_prototype::aspects::hints::{read_all_hints, write_all_hints};
use aspectation_prototype::aspects::register::{AnalyticsWrapper, AspectWrapper, HintWrapper};
use aspectation_prototype::aspects::side_effect::model::{
    SideEffectAnalytics, SideEffectAnalyticsValue, SideEffectAspect, SideEffectHint,
};
use aspectation_prototype::semantic::model::FuncName;
use aspectation_prototype::semantic::model::{FnWithAnalytics, FunctionSignature};
use aspectation_prototype::utils::{create_all_paths, remove_all_paths};
use aspectation_prototype::SETTINGS;
use std::error::Error;
use std::rc::Rc;

fn set_path() -> Result<(), Box<dyn Error>> {
    SETTINGS
        .write()?
        .set("project_path", "tests_output/side_effects_aspect")?;
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
fn read_hints_equal_to_written() {
    setup();
    let aspects = vec![AspectWrapper::SideEffect(Rc::new(SideEffectAspect {}))];
    let mut fn_with_aspects = HashMap::new();
    let name = FuncName::new("test_func");
    fn_with_aspects.insert(name.clone(), {
        let mut analytics = Vec::new();
        analytics.push(HintWrapper::SideEffect(Rc::new(
            SideEffectHint::NoSideEffects,
        )));
        analytics
    });
    let name2 = FuncName::new("test_func2");
    fn_with_aspects.insert(name2.clone(), {
        let mut analytics = Vec::new();
        analytics.push(HintWrapper::SideEffect(Rc::new(
            SideEffectHint::AllowedSideEffects(SideEffectAnalytics {
                values: {
                    let mut values = HashSet::new();
                    values.insert(SideEffectAnalyticsValue::ConsoleInput);
                    values.insert(SideEffectAnalyticsValue::ConsoleOutput);
                    values
                },
            }),
        )));
        analytics
    });
    write_all_hints(&fn_with_aspects, &aspects);
    let read_hints = read_all_hints(&aspects);
    assert_eq!(
        fn_with_aspects
            .get(&name)
            .unwrap()
            .iter()
            .collect::<Vec<&HintWrapper>>()
            .first()
            .unwrap(),
        read_hints
            .unwrap()
            .get(&name)
            .unwrap()
            .iter()
            .collect::<Vec<&HintWrapper>>()
            .first()
            .unwrap()
    );
}

#[test]
fn read_analytics_equal_to_written() {
    setup();
    let aspects = vec![AspectWrapper::SideEffect(Rc::new(SideEffectAspect {}))];
    let mut fn_with_aspects = HashMap::new();
    let name = FuncName::new("test_func");
    fn_with_aspects.insert(
        name.clone(),
        FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: name.clone(),
                args: Vec::new(),
            }),
            analytics: {
                let mut analytics = Vec::new();
                analytics.push(AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: {
                        let mut values = HashSet::new();
                        values.insert(SideEffectAnalyticsValue::None);
                        values
                    },
                })));
                analytics
            },
        },
    );
    let name2 = FuncName::new("test_func2");
    fn_with_aspects.insert(
        name2.clone(),
        FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: name2.clone(),
                args: Vec::new(),
            }),
            analytics: {
                let mut analytics = Vec::new();
                analytics.push(AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
                    values: {
                        let mut values = HashSet::new();
                        values.insert(SideEffectAnalyticsValue::ConsoleInput);
                        values.insert(SideEffectAnalyticsValue::ConsoleOutput);
                        values
                    },
                })));
                analytics
            },
        },
    );
    write_all_analytics(&fn_with_aspects, &aspects);
    let read_analytics = read_analytics_for_aspects(&aspects);
    assert_eq!(
        fn_with_aspects
            .get(&name)
            .unwrap()
            .analytics
            .iter()
            .collect::<Vec<&AnalyticsWrapper>>()
            .first()
            .unwrap(),
        read_analytics
            .unwrap()
            .get(&name)
            .unwrap()
            .iter()
            .collect::<Vec<&AnalyticsWrapper>>()
            .first()
            .unwrap()
    );
}
