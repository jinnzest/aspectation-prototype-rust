use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use aspects::analytics_parser::AnalyticsParser;
use aspects::model::*;
use aspects::register::{to_analytics_trait, to_aspect_trait, AnalyticsWrapper, AspectWrapper};
use aspects::utils::{read_all, remap};
use files::*;
use parsing::lines_reader::LinesReader;
use parsing::model::Errors;
use parsing::tokenizer::Tokenizer;
use paths::analytics_file_path;
use semantic::model::*;

pub fn read_analytics_from_file<A: Clone>(
    file_name: &str,
    analytics_parser: &impl Fn(&mut Tokenizer) -> Result<A, Errors>,
) -> Result<HashMap<FuncName, A>, Errors> {
    read_from_file(&analytics_file_path(file_name), &|body: &str| {
        parse_analytics_body(analytics_parser, body)
    })
}

fn parse_analytics_body<A: Clone>(
    analytics_parser: &impl Fn(&mut Tokenizer) -> Result<A, Errors>,
    body: &str,
) -> Result<HashMap<FuncName, A>, Errors> {
    let mut reader = LinesReader::new(body);
    let mut parser = AnalyticsParser::new(&mut reader);
    parser.parse(&analytics_parser)
}

pub fn read_analytics_for_aspects(
    aspects: &[AspectWrapper],
) -> Result<HashMap<FuncName, AnalyticsFields>, Errors> {
    read_all(aspects, &|a: Rc<dyn Aspect>| a.read_analytics())
}

pub fn inject_analytics(
    analytics: HashMap<FuncName, AnalyticsFields>,
    functions: &HashMap<FuncName, &Function>,
) -> HashMap<FuncName, FnWithAnalytics> {
    let mut enriched_analytics = HashMap::new();
    functions.iter().for_each(|(_, f)| {
        inject_for_func(&mut enriched_analytics, &analytics, *f);
    });
    enriched_analytics
}

fn inject_for_func(
    enriched_analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    analytics: &HashMap<FuncName, Vec<AnalyticsWrapper>>,
    f: &Function,
) {
    enriched_analytics.insert(
        f.name.clone(),
        FnWithAnalytics {
            sig: Rc::new(FunctionSignature {
                name: f.name.clone(),
                args: f.args.clone(),
            }),
            analytics: analytics.get(&f.name).unwrap_or(&Vec::new()).clone(),
        },
    );
}

pub fn generate_analytics(
    source_code_funcs: &HashMap<FuncName, &Function>,
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    aspects: &[AspectWrapper],
) {
    generate_analytics_for_functions(&source_code_funcs, analytics, aspects)
}

fn generate_analytics_for_functions(
    source_code_funcs: &HashMap<FuncName, &Function>,
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    aspects: &[AspectWrapper],
) {
    for f in source_code_funcs.values() {
        aspects
            .iter()
            .for_each(|a| to_aspect_trait(a).gen_analytics(f, analytics, source_code_funcs));
    }
}

pub fn write_all_analytics(
    analytics: &HashMap<FuncName, FnWithAnalytics>,
    aspects: &[AspectWrapper],
) {
    let mut analytics_fields = analytics
        .values()
        .flat_map(|fa| {
            let name = fa.sig.name.clone();
            fa.analytics.iter().map(move |a| (name.clone(), a.clone()))
        })
        .collect();
    sort_analytics_fields(&mut analytics_fields);
    aspects.iter().for_each(|a| {
        to_aspect_trait(a).write_analytics(&analytics_fields);
    });
}

fn sort_analytics_fields(analytics_results: &mut Vec<(FuncName, AnalyticsWrapper)>) {
    analytics_results.sort_by(|(name1, _), (name2, _)| name1.partial_cmp(name2).unwrap());
}

pub fn write_analytics_wrapper(
    name: &str,
    analytics_fields: &[(FuncName, Rc<AnalyticsWrapper>)],
    legenda: &str,
) {
    let converted_analytics_fields: Vec<(FuncName, Rc<dyn Analytics>)> = analytics_fields
        .iter()
        .map(|(f, aw)| {
            let analytics = to_analytics_trait(aw);
            (f.clone(), analytics)
        })
        .collect();
    write_analytics(name, converted_analytics_fields.as_slice(), legenda);
}

pub fn write_analytics(
    file_name: &str,
    analytics: &[(FuncName, Rc<dyn Analytics>)],
    legenda: &str,
) {
    write_to_file(&analytics_file_path(file_name), || {
        let mut result = "".to_owned();
        for (name, analytics) in analytics {
            result = format!("{} = {}\n{}", name, analytics, result);
        }
        result = format!("{}\nlegenda: {}\n", result, legenda);
        result
    });
}

pub fn remap_analytics(
    analytics: HashMap<FuncName, AnalyticsFields>,
    old_to_new_names: &HashMap<FuncName, FuncName>,
) -> HashMap<FuncName, AnalyticsFields> {
    remap(analytics, old_to_new_names)
}

pub fn remove_updated_functions<'a>(
    analytics: HashMap<FuncName, AnalyticsFields>,
    old_hashes: &HashMap<SemanticHash, FuncName>,
    generated_hashes: &'a HashMap<FuncName, SemanticHash>,
) -> HashMap<FuncName, AnalyticsFields> {
    let updated_functions: HashSet<&FuncName> = old_hashes
        .iter()
        .filter(|(_, f)| !generated_hashes.contains_key(f))
        .map(|(_, f)| f)
        .collect();
    analytics
        .into_iter()
        .filter(|(n, _)| !updated_functions.contains(n))
        .collect()
}

pub fn extract_concrete_analytics<R>(
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    filter_analytics: &impl Fn(&AnalyticsWrapper) -> Option<Rc<R>>,
) -> HashMap<FuncName, Rc<R>> {
    analytics
        .iter()
        .map(|(f, aw)| {
            (
                f,
                aw.analytics
                    .iter()
                    .filter_map(&filter_analytics)
                    .collect::<Vec<Rc<R>>>(),
            )
        })
        .filter_map(|(f, a)| {
            if a.is_empty() {
                None
            } else {
                Some((f.clone(), a.first().unwrap().clone()))
            }
        })
        .collect()
}

pub fn merge_analytics<A: Clone>(
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    concrete_analytics: &mut HashMap<FuncName, A>,
    wrapper: &impl Fn(&A) -> AnalyticsWrapper,
    matcher_wrapper: &impl Fn(&AnalyticsWrapper, A) -> AnalyticsWrapper,
) {
    concrete_analytics.iter().for_each(|(func_name, a)| {
        let v: AnalyticsFields = match analytics.get(&func_name) {
            Some(af) => {
                if af.analytics.is_empty() {
                    vec![wrapper(a)]
                } else {
                    af.analytics
                        .iter()
                        .map(|aw| matcher_wrapper(aw, a.clone()))
                        .collect()
                }
            }
            None => vec![wrapper(a)],
        };
        let result = mk_result(analytics, source_code_funcs, func_name, v);
        analytics.insert(func_name.clone(), result);
    });
}

fn mk_result(
    analytics: &mut HashMap<FuncName, FnWithAnalytics>,
    source_code_funcs: &HashMap<FuncName, &Function>,
    func_name: &FuncName,
    v: Vec<AnalyticsWrapper>,
) -> FnWithAnalytics {
    FnWithAnalytics {
        sig: Rc::new(FunctionSignature {
            name: func_name.clone(),
            args: {
                match analytics.get(&func_name.clone()) {
                    Some(a) => a.sig.args.clone(),
                    None => match source_code_funcs.get(&func_name.clone()) {
                        Some(f) => f.args.clone(),
                        None => panic!("Func '{}' was not found in neither external functions nor source code ones", func_name)
                    }
                }
            },
        }),
        analytics: v,
    }
}

pub fn extract_analytics_from_wrapper<A: Clone>(
    analytics: &[AnalyticsWrapper],
    f: &impl Fn(&AnalyticsWrapper) -> Option<Rc<A>>,
) -> Option<Rc<A>> {
    analytics
        .iter()
        .filter_map(f)
        .collect::<Vec<Rc<A>>>()
        .first()
        .cloned()
}

#[cfg(test)]
mod test_analytics {
    use super::*;
    use aspects::side_effect::model::SideEffectAnalytics;
    use parsing::model::Error;
    use parsing::tokenizer::{PosTok, Tok};

    #[test]
    pub fn remap_analytics_test() {
        let mut analytics = HashMap::new();
        let analytics_vec = vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
            values: HashSet::new(),
        }))];
        analytics.insert(FuncName::new("old_name"), analytics_vec.clone());
        analytics.insert(FuncName::new("other_name"), vec![]);
        let mut old_to_new_names = HashMap::new();
        old_to_new_names.insert(FuncName::new("old_name"), FuncName::new("new_name"));

        let result_analytics = remap_analytics(analytics, &old_to_new_names);

        assert_eq!(
            result_analytics.get(&FuncName::new("other_name")).unwrap(),
            &vec![]
        );
        assert_eq!(
            result_analytics.get(&FuncName::new("new_name")).unwrap(),
            &analytics_vec
        );
    }

    #[test]
    pub fn remove_updated_functions_test() {
        let mut analytics = HashMap::new();
        let analytics_vec = vec![AnalyticsWrapper::SideEffect(Rc::new(SideEffectAnalytics {
            values: HashSet::new(),
        }))];
        analytics.insert(FuncName::new("func_to_remove"), analytics_vec.clone());
        analytics.insert(
            FuncName::new("not_modified_func_hash"),
            analytics_vec.clone(),
        );
        let mut old_hashes = HashMap::new();
        old_hashes.insert(
            SemanticHash::new("func_to_remove_hash"),
            FuncName::new("func_to_remove"),
        );
        old_hashes.insert(
            SemanticHash::new("not_modified_func_hash"),
            FuncName::new("not_modified_func"),
        );
        let mut generated_hashes = HashMap::new();
        generated_hashes.insert(
            FuncName::new("not_modified_func"),
            SemanticHash::new("not_modified_func_hash"),
        );

        let cleaned_analytics = remove_updated_functions(analytics, &old_hashes, &generated_hashes);

        assert_eq!(cleaned_analytics.len(), 1);
        assert_eq!(
            cleaned_analytics.get(&FuncName::new("not_modified_func_hash")),
            Some(&analytics_vec)
        );
    }

    fn analytics_parser(tokenizer: &mut Tokenizer) -> Result<String, Vec<Error>> {
        match tokenizer.curr() {
            Ok(PosTok {
                tok: Tok::Ident(ident),
                ..
            }) => {
                tokenizer.next();
                Ok(ident)
            }
            Ok(PosTok {
                tok: unexpected, ..
            }) => Ok(format!("{}", unexpected)),
            Err(err) => Err(err),
        }
    }

    #[test]
    pub fn parse_analytics_body_test() {
        let parsed = parse_analytics_body(&analytics_parser, "a=a1\nb=b1\nc=c1");
        let map = parsed.unwrap();
        assert_eq!(map.get(&FuncName::new("a")).unwrap(), "a1");
        assert_eq!(map.get(&FuncName::new("b")).unwrap(), "b1");
        assert_eq!(map.get(&FuncName::new("c")).unwrap(), "c1");
    }

    #[test]
    pub fn parse_analytics_broken_body_test() {
        let result = parse_analytics_body(&analytics_parser, "a=a1\nsome broken line\nc=c1");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.err().unwrap().get(0).unwrap().message,
            "Expected: '='\nGot: 'broken' at 2:6"
        )
    }
}
