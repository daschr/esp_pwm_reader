use std::env;
use std::io::Error as IoError;

fn main() -> Result<(), IoError> {
    if let Ok(path) = env::var("OUT_DIR") {
        env::set_current_dir(path)?;
    }

    embuild::espidf::sysenv::output();

    Ok(())
}
