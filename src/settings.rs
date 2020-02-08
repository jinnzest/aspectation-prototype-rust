use SETTINGS;

use std::error::Error;

pub fn init_settings() {
    if let Result::Err(err) = init_settings_res() {
        panic!("{}", err)
    }
}

pub fn init_settings_res() -> Result<(), Box<dyn Error>> {
    SETTINGS.write()?.set("project_path", "project")?;
    Ok(())
}
