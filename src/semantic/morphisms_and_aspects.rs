#![allow(dead_code)]
extern crate crypto;

use aspects::analytics::*;
use aspects::hints::*;
use aspects::model::HintFields;
use aspects::register::{to_aspect_trait, AspectWrapper};
use morphism_parser::parser::Constr;
use parsing::model::{Error, Errors, Loc};
use semantic::core_morphisms::mk_semantic_tree;
use semantic::hashes::*;
use semantic::model::FuncName;
use semantic::model::*;
use semantic::utils::extract_functions;
use std::collections::HashMap;
use std::rc::Rc;
use utils::stop_on_recursion;

pub fn handle_morphisms_and_aspects(
    source_file_name: &str,
    constrs: &[Constr],
    external_funcs_analytics: &HashMap<FuncName, FnWithAnalytics>,
    aspects: &[AspectWrapper],
) -> Result<HashMap<Rc<FunctionSignature>, FnWithHints>, Errors> {
    reset_disabled_hints(aspects);
    let loaded_hints = read_all_hints(aspects)?;
    let constructions = mk_semantic_tree(constrs, &loaded_hints, &external_funcs_analytics)?;
    stop_on_recursion(&constructions)?;
    let generated_hashes = generate_hashes(
        &constructions,
        &external_funcs_analytics.keys().cloned().collect(),
    );
    let loaded_analytics = read_analytics_for_aspects(aspects)?;
    let old_hashes = read_hashes(source_file_name)?;
    let old_to_new_names = map_old_to_new_names(&old_hashes, &generated_hashes);
    let remapped_hints = remap_hints(loaded_hints, &old_to_new_names);
    let remapped_analytics = remap_analytics(loaded_analytics, &old_to_new_names);
    let functions = extract_functions(&constructions);
    let mut analytics_with_sig = inject_analytics(remapped_analytics, &functions);
    let reference_map: Vec<(FuncName, FnWithAnalytics)> = external_funcs_analytics
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    analytics_with_sig.extend(reference_map);
    let mut enriched_hints = remapped_hints.clone();
    add_default_hints(&constructions, &mut enriched_hints, aspects);
    generate_analytics(&functions, &mut analytics_with_sig, aspects);
    let mut func_impl_map = HashMap::new();
    for item in constructions {
        match item {
            Construction::Function(f) => {
                let func_name = &f.name;
                let fn_with_asp = FnWithHints {
                    sig: Rc::new(FunctionSignature {
                        name: f.name.clone(),
                        args: f.args.clone(),
                    }),
                    body: f.body.clone(),
                    hints: enriched_hints.get(func_name).unwrap().clone(),
                };
                func_impl_map.insert(fn_with_asp.sig.clone(), fn_with_asp.clone());
            }
            _ => panic!("the construction is unsupported yet"),
        }
    }
    write_hashes(source_file_name, &generated_hashes);
    write_all_hints(&remapped_hints, aspects);
    write_all_analytics(&analytics_with_sig, aspects);
    let result = check_constraints(&remapped_hints, &analytics_with_sig, aspects);
    match result {
        Ok(_) => Ok(func_impl_map),
        Err(err) => Err(err),
    }
}

fn check_constraints(
    hints: &HashMap<FuncName, HintFields>,
    analytics: &HashMap<FuncName, FnWithAnalytics>,
    aspects: &[AspectWrapper],
) -> Result<(), Errors> {
    let result: Vec<Error> = hints.iter().fold(vec![], |mut acc, (n, h)| {
        let a = analytics.get(n).unwrap();
        aspects.iter().for_each(|asp| {
            let aspect = to_aspect_trait(asp);
            if aspect.aspect_enabled() {
                let result = aspect.check_constraint(h, a.analytics.as_slice());
                if !result.is_empty() {
                    acc.push(Error {
                        message: format!("{} for function '{}'", result, n),
                        loc: Loc {
                            pos: 0,
                            line: 0,
                            col: 0,
                        },
                    })
                }
            }
        });
        acc
    });
    if result.is_empty() {
        Ok(())
    } else {
        Err(result)
    }
}
