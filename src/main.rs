use log::{debug, info};
use std::fs::File;
use std::io::{Error, Read};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[cfg(test)]
#[macro_use]
mod test_helpers;
mod crontab;
mod schedule;

use crate::crontab::Crontab;

fn main() -> Result<(), Error> {
    pretty_env_logger::init_custom_env("CRONDY_LOG");
    let child_died = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGCHLD, Arc::clone(&child_died))?;
    info!("cron started");
    debug!("loading database");
    let crontab_path = std::env::var("CRONTAB").unwrap_or("/etc/crontab".to_owned());
    let mut system_crontab = File::open(crontab_path)?;
    let mut contents = String::new();
    system_crontab.read_to_string(&mut contents)?;
    let crontab = Crontab::parse(&contents)?;
    println!("{:?}", crontab);
    Ok(())
}
