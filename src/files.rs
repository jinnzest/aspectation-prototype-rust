use parsing::model;
use parsing::model::{Errors, Loc};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn read_from_file<R>(name: &str, f: &impl Fn(&str) -> Result<R, Errors>) -> Result<R, Errors> {
    let path = Path::new(name);
    let display = path.display();
    let mut buffer = String::new();
    match File::open(&path) {
        Ok(mut fl) => match fl.read_to_string(&mut buffer) {
            Err(why) => Err(vec![model::Error {
                message: format!("can't read from {}: {}", display, why),
                loc: Loc {
                    pos: 0,
                    line: 0,
                    col: 0,
                },
            }]),
            Ok(_) => {
                //                println!("File BUF: {:?}", buffer);
                f(&buffer)
            }
        },
        Err(err) => {
            if err.raw_os_error() == Some(2) {
                f("")
            } else {
                Err(vec![model::Error {
                    message: format!(
                        "can't open file '{}' because of error '{}'",
                        path.display(),
                        err
                    ),
                    loc: Loc {
                        pos: 0,
                        line: 0,
                        col: 0,
                    },
                }])
            }
        }
    }
}

pub fn write_to_file<F>(name: &str, f: F)
where
    F: Fn() -> String,
{
    let path = Path::new(name);
    let display = path.display();

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    if let Err(why) = file.write_all(f().as_bytes()) {
        panic!("couldn't write to {}: {}", display, why)
    }
}
