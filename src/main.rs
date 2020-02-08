#![allow(clippy::implicit_hasher)]
#![allow(clippy::missing_safety_doc)]

#[macro_use]
extern crate lazy_static;
extern crate aspectation_prototype;
extern crate config;
extern crate llvm_sys;
extern crate proc_macro;

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

use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

use aspects::model::Aspects;
use generator::code_generator::*;
use paths::*;
use semantic::morphisms_and_aspects::*;
use utils::*;

use aspects::complexity::model::ComplexityAspect;
use aspects::register::AspectWrapper;
use aspects::side_effect::model::SideEffectAspect;
use config::Config;
use generator::llvm_wrapper::LLVMWrapper;
use generator::native_funcs::{to_native_funcs_map, NativeFuncsGenerator};
use morphism_parser::parser::{Constr, MorphismParser};
use parsing::lines_reader::LinesReader;
use parsing::model::Error;
use parsing::model::Errors;
use semantic::model::{FnWithHints, FunctionSignature};
use settings::init_settings;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::RwLock;

lazy_static! {
    static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}

fn main() {
    init_settings();

    create_all_paths();
    let enabled_analytics = mk_aspects();

    let source_file_name = "main";

    let mut file = File::open(src_file(source_file_name));

    match file {
        Ok(ref mut f) => {
            let mut text = String::new();
            let result = f.read_to_string(&mut text);
            match result {
                Ok(_) => {
                    let source = format!("{}\n", text);
                    let mut reader = LinesReader::new(&source);
                    let mut parser = MorphismParser::new(&mut reader);
                    let parsing_result = parser.parse();
                    parser.go_to_eof();
                    let lines = parser.line_pos();
                    handle_parsing_result(
                        &source,
                        &enabled_analytics,
                        source_file_name,
                        parsing_result,
                        &lines,
                    )
                }
                Err(err) => println!("error reading file: {}", err),
            }
        }
        Err(err) => println!("{:?}", err),
    }
}

fn handle_parsing_result(
    source: &str,
    enabled_analytics: &[AspectWrapper],
    source_file_name: &str,
    res: Result<Vec<Constr>, Errors>,
    lines: &[usize],
) {
    match res {
        Ok(constrs) => {
            handle_parsed_constructions(&enabled_analytics, source_file_name, &constrs);
        }
        Err(err) => {
            handle_parsing_errors(source, &err, lines);
            std::process::exit(-1);
        }
    }
}

fn handle_parsing_errors(source: &str, errors: &[Error], lines: &[usize]) {
    println!("COMPILATION ERRORS: ");
    for err in errors {
        let s = &source[lines[err.loc.line - 1]..lines[err.loc.line]];
        println!("{}\n{}", err.message, s);
        let pointer = (1..err.loc.col).fold("".to_owned(), |acc, _| format!("{} ", acc));
        println!("{}^", pointer);
    }
}

fn handle_parsed_constructions(
    enabled_analytics: &[AspectWrapper],
    source_file_name: &str,
    constrs: &[Constr],
) {
    let mut llvm = LLVMWrapper::default();
    let native_funcs_gen = NativeFuncsGenerator::new(&mut llvm);
    let native_funcs = native_funcs_gen.native_funcs();
    let mut generator = CodeGenerator::new(&mut llvm, &native_funcs);
    let native_funcs_map = to_native_funcs_map(&native_funcs);
    let result = handle_morphisms_and_aspects(
        source_file_name,
        constrs,
        &native_funcs_map,
        &enabled_analytics,
    );
    handle_analytics_result(&mut generator, result)
}

fn handle_analytics_result(
    generator: &mut CodeGenerator,
    funcs_with_hints: Result<HashMap<Rc<FunctionSignature>, FnWithHints>, Errors>,
) {
    match funcs_with_hints {
        Ok(fn_with_hints) => {
            generator.gen_program(fn_with_hints);
            mk_executable();
            std::process::exit(0);
        }
        Err(errors) => {
            println!("Analytics errors:");
            errors.iter().rev().for_each(|e| println!("{}", e.message));
            std::process::exit(-1);
        }
    }
}

fn mk_aspects() -> Aspects {
    vec![
        AspectWrapper::SideEffect(Rc::new(SideEffectAspect {})),
        //        AspectWrapper::ResourcesManagement(Rc::new(ResourcesManagementAspect {})),
        AspectWrapper::Complexity(Rc::new(ComplexityAspect {})),
    ]
}

pub fn mk_executable() -> bool {
    println!("Generating executable...");
    let cc = Command::new("cc")
        .arg("./target/output.o")
        .arg("./libtommath/libtommath.a")
        .arg("-o")
        .arg("./target/out")
        .output()
        .expect("");
    println!("status: {}", cc.status);
    println!("stdout: {}", String::from_utf8_lossy(&cc.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&cc.stderr));
    cc.stderr.is_empty()
}
