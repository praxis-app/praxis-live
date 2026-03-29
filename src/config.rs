use std::{env, error::Error, io};

pub(crate) fn required_env(name: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    env::var(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be set before starting the server."),
        )
        .into()
    })
}
