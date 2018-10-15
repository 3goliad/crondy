use log::info;
use std::io::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod db;
use crate::db::Db;

fn main() -> Result<(), Error> {
    pretty_env_logger::init_custom_env("CRONDY_LOG");
    let child_died = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGCHLD, Arc::clone(&child_died))?;
    info!("cron started");
    let mut db = Db::new();
    Ok(())
}
