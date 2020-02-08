use aspects::analytics::write_analytics_wrapper;
use aspects::model::{Aspect, HintFields};
use aspects::register::{AnalyticsWrapper, HintWrapper};
use aspects::side_effect::analytics::{generate_side_effect_analytics, read_side_effect_analytics};
use aspects::side_effect::constraint::side_effect_constraint;
use aspects::side_effect::hints::{read_side_effect_hints, write_side_effect_hints};
use aspects::side_effect::model::{SideEffectAspect, SideEffectHint};
use parsing::model::{Error, Errors, Loc};
use semantic::model::FnWithAnalytics;
use semantic::model::*;
use std::collections::HashMap;
use std::rc::Rc;

impl Aspect for SideEffectAspect {
    fn name(&self) -> String {
        SideEffectAspect::name()
    }

    fn read_hints(&self) -> Result<HashMap<FuncName, HintWrapper>, Errors> {
        let result = read_side_effect_hints();
        match result {
            Ok(v) => Ok(v
                .iter()
                .map(|(f, h)| {
                    let hint = HintWrapper::SideEffect(Rc::new(h.clone()));
                    (f.clone(), hint)
                })
                .collect()),
            Err(mut err) => {
                err.push(Error {
                    message: "Reading side effects hints error: ".to_owned(),
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
        write_side_effect_hints(&hints);
    }

    fn write_analytics(&self, analytics: &[(FuncName, AnalyticsWrapper)]) {
        let side_effect_analytics: Vec<(FuncName, Rc<AnalyticsWrapper>)> = analytics
            .iter()
            .filter_map(|(n, hw)| match hw {
                AnalyticsWrapper::SideEffect(_) => Some((n.clone(), Rc::new(hw.clone()))),
                _ => None,
            })
            .collect();
        write_analytics_wrapper(
            &SideEffectAspect::name(),
            side_effect_analytics.as_slice(),
            "'any' means that any side effect is allowed, 'none' means that no effects are allowed",
        );
    }

    fn read_analytics(&self) -> Result<HashMap<FuncName, AnalyticsWrapper>, Errors> {
        let result = read_side_effect_analytics();
        match result {
            Ok(v) => Ok(v
                .iter()
                .map(|(n, a)| {
                    let wrapper = AnalyticsWrapper::SideEffect(Rc::new(a.clone()));
                    (n.clone(), wrapper)
                })
                .collect()),
            Err(mut err) => {
                err.push(Error {
                    message: "Reading side effects analytics error: ".to_owned(),
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
        generate_side_effect_analytics(f, analytics, source_code_funcs)
    }

    fn default_hint(&self, _f: &Function) -> HintWrapper {
        HintWrapper::SideEffect(Rc::new(SideEffectHint::AnySideEffect))
    }

    fn check_constraint(&self, hints: &[HintWrapper], analytics: &[AnalyticsWrapper]) -> String {
        side_effect_constraint(hints, analytics)
    }

    fn aspect_enabled(&self) -> bool {
        true
    }
}
