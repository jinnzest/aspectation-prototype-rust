#![allow(clippy::implicit_hasher)]
#![allow(clippy::missing_safety_doc)]

#[macro_use]
extern crate lazy_static;
extern crate config;
extern crate crypto;
extern crate llvm_sys;

pub mod aspects;
pub mod files;
pub mod generator;
pub mod model;
pub mod morphism_parser;
pub mod parsing;
pub mod paths;
pub mod semantic;
pub mod settings;
pub mod utils;

use config::Config;
use std::sync::RwLock;

lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}
