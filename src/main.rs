use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::process;
use std::fs::File;
use std::io::Read;
use std::time::SystemTime;

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

    // don't forget to check DST!!!
    let mut clock_time = SystemTime::now();

    crontab.run_reboot_jobs();

    let mut time_running = clock_time;
    let mut virtual_time = clock_time;
    /*
     * too many clocks, not enough time (Al. Einstein)
     * These clocks are in minutes since the epoch (time()/60).
     * virtual_time: is the time it *would* be if we woke up
     * promptly and nobody ever changed the clock. It is
     * monotonically increasing... unless a timejump happens.
     * At the top of the loop, all jobs for 'virtual_time' have run.
     * time_running: is the time we last awakened.
     * clock_time: is the time when set_time was last called.
     */
    loop {
        //		time_min timeDiff;
        //		int wakeupKind;
        //
        //		/* ... wait for the time (in minutes) to change ... */
        //		do {
        //			cron_sleep(timeRunning + 1);
        //			set_time(FALSE);
        //		} while (clockTime == timeRunning);
        //		timeRunning = clockTime;
        let target = time_running.duration_since(SystemTime::UNIX_EPOCH) + one_minute;
        let wait = (target - SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)) + one_second;
        debug!("sleeping (target: {}, wait: {})", target, wait);
        thread::sleep(wait);
        //
        //		check_orphans(&database);
        //		load_database(&database);
        //
        //		/*
        //		 * ... calculate how the current time differs from
        //		 * our virtual clock. Classify the change into one
        //		 * of 4 cases
        //		 */
        //		timeDiff = timeRunning - virtualTime;
        //
        //		Debug(DSCH, ("[%d] pulse: %d = %d - %d\n",
        //            	    getpid(), timeDiff, timeRunning, virtualTime));
        //
        //		/* shortcut for the most common case */
        //		if (timeDiff == 1) {
        //			virtualTime = timeRunning;
        //			find_jobs(virtualTime, &database, TRUE, TRUE);
        //		} else {
        //			wakeupKind = -1;
        //			if (timeDiff > -(3*MINUTE_COUNT))
        //				wakeupKind = 0;
        //			if (timeDiff > 0)
        //				wakeupKind = 1;
        //			if (timeDiff > 5)
        //				wakeupKind = 2;
        //			if (timeDiff > (3*MINUTE_COUNT))
        //				wakeupKind = 3;
        //
        //			switch (wakeupKind) {
        //			case 1:
        //				/*
        //				 * case 1: timeDiff is a small positive number
        //				 * (wokeup late) run jobs for each virtual minute
        //				 * until caught up.
        //				 */
        //				Debug(DSCH, ("[%d], normal case %d minutes to go\n",
        //				    getpid(), timeRunning - virtualTime))
        //				do {
        //					if (job_runqueue())
        //						sleep(10);
        //					virtualTime++;
        //					find_jobs(virtualTime, &database, TRUE, TRUE);
        //				} while (virtualTime< timeRunning);
        //				break;
        //
        //			case 2:
        //				/*
        //				 * case 2: timeDiff is a medium-sized positive number,
        //				 * for example because we went to DST run wildcard
        //				 * jobs once, then run any fixed-time jobs that would
        //				 * otherwise be skipped if we use up our minute
        //				 * (possible, if there are a lot of jobs to run) go
        //				 * around the loop again so that wildcard jobs have
        //				 * a chance to run, and we do our housekeeping
        //				 */
        //				Debug(DSCH, ("[%d], DST begins %d minutes to go\n",
        //				    getpid(), timeRunning - virtualTime))
        //				/* run wildcard jobs for current minute */
        //				find_jobs(timeRunning, &database, TRUE, FALSE);
        //
        //				/* run fixed-time jobs for each minute missed */
        //				do {
        //					if (job_runqueue())
        //						sleep(10);
        //					virtualTime++;
        //					find_jobs(virtualTime, &database, FALSE, TRUE);
        //					set_time(FALSE);
        //				} while (virtualTime< timeRunning &&
        //				    clockTime == timeRunning);
        //				break;
        //
        //			case 0:
        //				/*
        //				 * case 3: timeDiff is a small or medium-sized
        //				 * negative num, eg. because of DST ending just run
        //				 * the wildcard jobs. The fixed-time jobs probably
        //				 * have already run, and should not be repeated
        //				 * virtual time does not change until we are caught up
        //				 */
        //				Debug(DSCH, ("[%d], DST ends %d minutes to go\n",
        //				    getpid(), virtualTime - timeRunning))
        //				find_jobs(timeRunning, &database, TRUE, FALSE);
        //				break;
        //			default:
        //				/*
        //				 * other: time has changed a *lot*,
        //				 * jump virtual time, and run everything
        //				 */
        //				Debug(DSCH, ("[%d], clock jumped\n", getpid()))
        //				virtualTime = timeRunning;
        //				find_jobs(timeRunning, &database, TRUE, TRUE);
        //			}
        //		}
        //		/* jobs to be run (if any) are loaded. clear the queue */
        //		job_runqueue();

    }
}
