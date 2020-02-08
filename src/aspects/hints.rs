use aspects::hint_parser::HintsParser;
use aspects::model::*;
use aspects::register::{to_aspect_trait, to_hint_trait, AspectWrapper, HintWrapper};
use aspects::utils::{read_all, remap};
use files::{read_from_file, write_to_file};
use parsing::lines_reader::LinesReader;
use parsing::model::Errors;
use parsing::tokenizer::Tokenizer;
use paths::{hints_file_path, hints_path};
use semantic::model::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

pub fn add_default_hints(
    constructions: &[Construction],
    hints: &mut HashMap<FuncName, HintFields>,
    aspects: &[AspectWrapper],
) {
    constructions.iter().for_each(|c| {
        if let Construction::Function(f) = c {
            let mut default_hints: HashSet<HintWrapper> = aspects
                .iter()
                .map(|a| to_aspect_trait(a).default_hint(&f))
                .collect();
            let hint = hints
                .entry(f.name.clone())
                .or_insert_with(|| default_hints.iter().cloned().collect());
            hint.iter().for_each(|h| {
                default_hints.remove(h);
            });
            default_hints.iter().for_each(|h| hint.push(h.clone()));
        }
    });
}

pub fn read_all_hints(aspects: &[AspectWrapper]) -> Result<HashMap<FuncName, HintFields>, Errors> {
    read_all(aspects, &|a: Rc<dyn Aspect>| a.read_hints())
}

pub fn reset_disabled_hints(aspects: &[AspectWrapper]) {
    let enabled_aspect_names = aspects
        .iter()
        .map(|n| format!("{}.hnt", to_aspect_trait(n).name()))
        .collect::<HashSet<String>>();
    for entry in std::fs::read_dir(std::path::Path::new(&hints_path())).unwrap() {
        let unwrapped = entry.unwrap();
        let file_name = unwrapped
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        if !enabled_aspect_names.contains(&file_name) {
            println!("FILE: {}", file_name);
            match std::fs::remove_file(unwrapped.path()) {
                Result::Ok(_) => println!("deleted file {}", unwrapped.path().to_str().unwrap()),
                Result::Err(err) => panic!(
                    "file {} can't be removed because of the error: {}",
                    unwrapped.path().to_str().unwrap(),
                    err
                ),
            }
        }
    }
}

pub fn write_all_hints(hints: &HashMap<FuncName, HintFields>, aspects: &[AspectWrapper]) {
    let mut hint_fields: Vec<(FuncName, Vec<HintWrapper>)> =
        hints.iter().map(|(n, h)| (n.clone(), h.clone())).collect();
    sort_hint_fields(&mut hint_fields);
    aspects.iter().for_each(|a| {
        to_aspect_trait(a).write_hints(&hint_fields);
    });
}

fn sort_hint_fields(analytics_results: &mut Vec<(FuncName, HintFields)>) {
    analytics_results.sort_by(|(name1, _), (name2, _)| name1.partial_cmp(name2).unwrap());
}

pub fn remap_hints(
    hints: HashMap<FuncName, HintFields>,
    old_to_new_names: &HashMap<FuncName, FuncName>,
) -> HashMap<FuncName, HintFields> {
    remap(hints, old_to_new_names)
}

pub fn write_hints(
    file_name: &str,
    hint_fields: &[(FuncName, HintFields)],
    wrap: &impl Fn(&HintWrapper) -> Option<Rc<HintWrapper>>,
) {
    let hints = extract_hints(hint_fields, wrap);
    if !hints.is_empty() {
        write_to_file(&hints_file_path(file_name), || {
            let mut result = "".to_owned();
            hints.iter().for_each(|(name, hint)| {
                result = format!("{} <- {}\n{}", name, to_hint_trait(&hint), result);
            });
            result
        });
    }
}

pub fn read_hints<H: Clone>(
    file_name: &str,
    hint_parser: &impl Fn(&mut Tokenizer) -> Result<H, Errors>,
) -> Result<HashMap<FuncName, H>, Errors> {
    read_from_file(&hints_file_path(file_name), &|body: &str| {
        hints_from_str(&hint_parser, body)
    })
}

fn extract_hints(
    hints: &[(FuncName, HintFields)],
    wrap: &impl Fn(&HintWrapper) -> Option<Rc<HintWrapper>>,
) -> Vec<(FuncName, Rc<HintWrapper>)> {
    hints
        .iter()
        .filter_map(|(n, hv)| {
            let result: Vec<Rc<HintWrapper>> = hv.iter().filter_map(wrap).collect();
            match result.first() {
                Some(res) => Some((n.clone(), res.clone())),
                None => None,
            }
        })
        .collect()
}

fn hints_from_str<H: Clone>(
    hint_parser: &impl Fn(&mut Tokenizer) -> Result<H, Errors>,
    body: &str,
) -> Result<HashMap<FuncName, H>, Errors> {
    let mut reader = LinesReader::new(body);
    let mut parser = HintsParser::new(&mut reader);
    parser.parse(&hint_parser)
}

pub fn extract_hints_from_wrapper<H: Clone>(
    hints: &[HintWrapper],
    f: &impl Fn(&HintWrapper) -> Option<Rc<H>>,
) -> Option<Rc<H>> {
    hints
        .iter()
        .filter_map(f)
        .collect::<Vec<Rc<H>>>()
        .first()
        .cloned()
}

#[cfg(test)]
mod test_hints {
    use aspects::hints::add_default_hints;
    use aspects::register::{AspectWrapper, HintWrapper};
    use aspects::side_effect::model::{SideEffectAspect, SideEffectHint};
    use semantic::model::FuncName;
    use semantic::model::*;
    use std::borrow::Borrow;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    pub fn add_missed_default_side_effect_hint_test() {
        let aspects = vec![AspectWrapper::SideEffect(Rc::new(SideEffectAspect {}))];
        let mut hints = HashMap::new();
        add_default_hints(
            vec![Construction::Function(Function {
                name: FuncName::new("test_func"),
                args: Vec::new(),
                body: Expression::SubExpression(vec![]),
            })]
            .as_slice(),
            &mut hints,
            &aspects,
        );
        assert_eq!(
            hints
                .get(&FuncName::new("test_func"))
                .unwrap()
                .iter()
                .find(|h| {
                    match h {
                        HintWrapper::SideEffect(se) => match se.borrow() {
                            SideEffectHint::AnySideEffect => true,
                            _ => false,
                        },
                        _ => false,
                    }
                })
                .is_some(),
            true
        );
    }
}
