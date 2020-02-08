use aspects::analytics::write_analytics_wrapper;
use aspects::complexity::analytics::{generate_complexity_analytics, read_complexity_analytics};
use aspects::complexity::constraint::check_complexity_constraint;
use aspects::complexity::hints::{read_complexity_hints, write_complexity_hints};
use aspects::complexity::model::{ComplexityAspect, ComplexityHint, ComplexityHintValue};
use aspects::model::{Aspect, HintFields};
use aspects::register::{AnalyticsWrapper, HintWrapper};
use parsing::model::{Error, Errors, Loc};
use semantic::model::FnWithAnalytics;
use semantic::model::*;
use std::collections::HashMap;
use std::rc::Rc;

impl Aspect for ComplexityAspect {
    fn name(&self) -> String {
        ComplexityAspect::name()
    }

    fn read_hints(&self) -> Result<HashMap<FuncName, HintWrapper>, Errors> {
        match read_complexity_hints() {
            Ok(v) => Ok(v
                .iter()
                .map(|(f, h)| {
                    let hint = HintWrapper::Complexity(Rc::new(h.clone()));
                    (f.clone(), hint)
                })
                .collect()),
            Err(mut err) => {
                err.push(Error {
                    message: "Reading complexity hints error: ".to_owned(),
                    loc: Loc {
                        pos: 0,
                        line: 0,
                        col: 0,
                    },
                });
                Err(err)
            }
        }
    }

    fn write_hints(&self, hints: &[(FuncName, HintFields)]) {
        write_complexity_hints(&hints);
    }

    fn write_analytics(&self, analytics: &[(FuncName, AnalyticsWrapper)]) {
        let side_effect_analytics = analytics
            .iter()
            .filter_map(|(n, hw)| match hw {
                AnalyticsWrapper::Complexity(_) => Some((n.clone(), Rc::new(hw.clone()))),
                _ => None,
            })
            .collect::<Vec<(FuncName, Rc<AnalyticsWrapper>)>>();
        write_analytics_wrapper(
            &ComplexityAspect::name(),
            side_effect_analytics.as_slice(),
            "where c is a constant, n is an integer number",
        );
    }

    fn read_analytics(&self) -> Result<HashMap<FuncName, AnalyticsWrapper>, Errors> {
        let result = read_complexity_analytics();
        match result {
            Ok(v) => Ok(v
                .iter()
                .map(|(n, a)| {
                    let wrapper = AnalyticsWrapper::Complexity(Rc::new(a.clone()));
                    (n.clone(), wrapper)
                })
                .collect()),
            Err(mut err) => {
                err.push(Error {
                    message: "Reading complexity analytics error: ".to_owned(),
                    loc: Loc {
                        pos: 0,
                        line: 0,
                        col: 0,
                    },
                });
                Err(err)
            }
        }
    }

    fn gen_analytics(
        &self,
        f: &Function,
        analytics: &mut HashMap<FuncName, FnWithAnalytics>,
        source_code_funcs: &HashMap<FuncName, &Function>,
    ) {
        generate_complexity_analytics(f, analytics, source_code_funcs)
    }

    fn default_hint(&self, f: &Function) -> HintWrapper {
        let values = f
            .args
            .iter()
            .map(|a| (a.clone(), ComplexityHintValue::Any))
            .collect();
        HintWrapper::Complexity(Rc::new(ComplexityHint { values }))
    }

    fn check_constraint(&self, hints: &[HintWrapper], analytics: &[AnalyticsWrapper]) -> String {
        check_complexity_constraint(hints, analytics)
    }

    fn aspect_enabled(&self) -> bool {
        true
    }
}
