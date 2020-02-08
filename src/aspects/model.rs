use aspects::register::{AnalyticsWrapper, AspectWrapper, HintWrapper};
use parsing::model::Errors;
use semantic::model::*;
use std::collections::HashMap;
use std::fmt::{Debug, Display};

pub trait Analytics: Display {}

pub trait Hint: Debug + Display {}

#[derive(Debug, Clone)]
pub struct Hash(String);

#[derive(Debug)]
pub struct ParseHintError();

pub type AnalyticsFields = Vec<AnalyticsWrapper>;

pub type HintFields = Vec<HintWrapper>;

pub type Aspects = Vec<AspectWrapper>;

pub trait Aspect {
    fn name(&self) -> String;
    fn read_hints(&self) -> Result<HashMap<FuncName, HintWrapper>, Errors>;
    fn write_hints(&self, hints: &[(FuncName, HintFields)]);
    fn write_analytics(&self, analytics: &[(FuncName, AnalyticsWrapper)]);
    fn read_analytics(&self) -> Result<HashMap<FuncName, AnalyticsWrapper>, Errors>;
    fn gen_analytics(
        &self,
        f: &Function,
        analytics: &mut HashMap<FuncName, FnWithAnalytics>,
        source_code_funcs: &HashMap<FuncName, &Function>,
    );
    fn default_hint(&self, f: &Function) -> HintWrapper;
    fn check_constraint(&self, h: &[HintWrapper], a: &[AnalyticsWrapper]) -> String;
    fn aspect_enabled(&self) -> bool;
}
