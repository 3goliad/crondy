use std::fs::File;
use std::io::Read;
use std::process;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use log::{debug, error, info};

#[cfg(test)]
#[macro_use]
mod test_helpers;
mod crontab;
mod schedule;

use crate::crontab::Crontab;

fn main() {
    pretty_env_logger::init_custom_env("CRONDY_LOG");

    let child_died = Arc::new(AtomicBool::new(false));
    match signal_hook::flag::register(signal_hook::SIGCHLD, Arc::clone(&child_died)) {
        Ok(_) => debug!("registered signal hook"),
        Err(error) => {
            error!("failed to register signal handlers: {}", error);
            process::exit(1);
        }
    }

    info!("starting up!");
    let crontab_path = std::env::var("CRONTAB").unwrap_or("/etc/crontab".to_owned());
    debug!("selected crontab: {}", &crontab_path);

    debug!("loading database");
    let mut crontab_file = File::open(&crontab_path).unwrap_or_else(|error| {
        use std::io::ErrorKind;
        match error.kind() {
            ErrorKind::NotFound => error!("could not find crontab file at path {}", &crontab_path),
            _ => error!("error opening crontab: {}", error),
        }
        process::exit(1);
    });

    let mut contents = String::new();
    crontab_file.read_to_string(&mut contents).unwrap();

    let crontab = Crontab::parse(&contents).unwrap_or_else(|error| {
        error!("error parsing crontab: {}", error);
        process::exit(1);
    });
    debug!("parsed crontab {:?}", crontab);

    crontab.validate().unwrap_or_else(|error| {
        error!("error validating crontab: {}", error);
        process::exit(1);
    });
    debug!("validated crontab");
}
