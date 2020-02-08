use aspects::complexity::model::{ComplexityAnalytics, ComplexityAspect, ComplexityHint};
use aspects::model::{Analytics, Aspect, Hint};
use aspects::side_effect::model::*;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum AspectWrapper {
    SideEffect(Rc<SideEffectAspect>),
    Complexity(Rc<ComplexityAspect>),
}

#[derive(Debug, Clone)]
pub enum AnalyticsWrapper {
    SideEffect(Rc<SideEffectAnalytics>),
    Complexity(Rc<ComplexityAnalytics>),
}

impl PartialEq for AnalyticsWrapper {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AnalyticsWrapper::SideEffect(left), AnalyticsWrapper::SideEffect(right)) => {
                *left.clone() == *right.clone()
            }
            (AnalyticsWrapper::Complexity(left), AnalyticsWrapper::Complexity(right)) => {
                *left.clone() == *right.clone()
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub enum HintWrapper {
    SideEffect(Rc<SideEffectHint>),
    Complexity(Rc<ComplexityHint>),
}

impl Hash for HintWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            HintWrapper::SideEffect(_) => "SideEffect".hash(state),
            HintWrapper::Complexity(_) => "Complexity".hash(state),
        }
    }
}

impl PartialEq for HintWrapper {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&HintWrapper::SideEffect(ref a), &HintWrapper::SideEffect(ref b)) => a == b,
            (&HintWrapper::Complexity(ref a), &HintWrapper::Complexity(ref b)) => a == b,
            _ => false,
        }
    }
}

pub fn to_analytics_trait(aw: &AnalyticsWrapper) -> Rc<dyn Analytics> {
    match aw {
        AnalyticsWrapper::SideEffect(a) => a.clone(),
        AnalyticsWrapper::Complexity(a) => a.clone(),
    }
}

pub fn to_aspect_trait(aw: &AspectWrapper) -> Rc<dyn Aspect> {
    match aw {
        AspectWrapper::SideEffect(a) => a.clone(),
        AspectWrapper::Complexity(a) => a.clone(),
    }
}

pub fn to_hint_trait(aw: &HintWrapper) -> Rc<dyn Hint> {
    match aw {
        HintWrapper::SideEffect(h) => h.clone(),
        HintWrapper::Complexity(h) => h.clone(),
    }
}
