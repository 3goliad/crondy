use log::debug;
use nom::{
    alt, alt_complete, call, char, complete, count, digit, digit1, eof, error_position, exact,
    many1, map, map_res, named, none_of, opt, preceded, rest, sep, separated_pair, space0, tag,
    terminated, tuple, tuple_parser, wrap_sep, ws,
};
use std::fs::File;
use std::io::{Error, Read};
use std::str::FromStr;

const SECONDS_PER_MINUTE: usize = 60;

const FIRST_MINUTE: usize = 0;
const LAST_MINUTE: usize = 59;
const MINUTE_COUNT: usize = (LAST_MINUTE - FIRST_MINUTE + 1);

const FIRST_HOUR: usize = 0;
const LAST_HOUR: usize = 23;
const HOUR_COUNT: usize = (LAST_HOUR - FIRST_HOUR + 1);

const FIRST_DAY_OF_MONTH: usize = 1;
const LAST_DAY_OF_MONTH: usize = 31;
const DAY_OF_MONTH_COUNT: usize = (LAST_DAY_OF_MONTH - FIRST_DAY_OF_MONTH + 1);

const FIRST_MONTH: usize = 1;
const LAST_MONTH: usize = 12;
const MONTH_COUNT: usize = (LAST_MONTH - FIRST_MONTH + 1);

/* note on DAY_OF_WEEK: 0 and 7 are both Sunday, for compatibility reasons. */
const FIRST_DAY_OF_WEEK: usize = 0;
const LAST_DAY_OF_WEEK: usize = 7;
const DAY_OF_WEEK_COUNT: usize = (LAST_DAY_OF_WEEK - FIRST_DAY_OF_WEEK + 1);

enum TimeDateField {
    Value(usize),
    Range(usize, usize, Option<usize>), // begin, end, step
    List(Vec<usize>),
    Star(Option<usize>), // step
}

enum CrontabLine {
    Entry(Entry),
    Env(String, String),
}

named!(step<&str, Option<usize>>, opt!(preceded!(char!('/'), integer)));
named!(integer<&str, usize>, map_res!(digit1, |s| usize::from_str(s)));
named!(parse_field<&str, TimeDateField>, alt!(
    map!(preceded!(char!('*'), step), |opt| TimeDateField::Star(opt)) |
    map!(tuple!(separated_pair!(integer, char!('-'), integer), step), |((begin, end), step)| TimeDateField::Range(begin, end, step))
));
named!(parse_when<&str, When>, map!(ws!(count!(parse_field, 5)), |fields| {

}));

named!(parse_schedule<&str, Schedule>, alt!(
    map!(tag!("@reboot"),
         |_| Schedule::Reboot) |
    map!(alt!(tag!("@yearly") | tag!("@annually")),
         |_| when!(0 0 1 1 *)) |
    map!(tag!("@monthly"),
         |_| when!(0 0 1 * *)) |
    map!(tag!("@weekly"),
         |_| when!(0 0 * * 0)) |
    map!(alt!(tag!("@daily") | tag!("@midnight")),
         |_| when!(0 0 * * *)) |
    map!(tag!("@hourly"),
         |_| when!(0 * * * *)) |
    parse_when
));
named!(parse_entry<&str, Option<CrontabLine>>, map!(tuple!(parse_schedule, parse_command), |(schedule, cmd)| Some(CrontabLine::Entry(
    Entry {
        envp: Vec::new(),
        cmd,
        schedule,
    }
))));
named!(
    parse_env<&str, Option<CrontabLine>>,
    map!(
        separated_pair!(many1!(none_of!("=")), char!('='), rest),
        |(name, value)| Some(CrontabLine::Env(name.into_iter().collect(), value.to_owned()))
    )
);
named!(parse_comment<&str, Option<CrontabLine>>, preceded!(char!('#'), map!(rest, |_| None)));

named!(
    parse_line<&str, Option<CrontabLine>>,
    exact!(preceded!(
        space0,
        alt_complete!(parse_entry | parse_env | parse_comment | map!(eof!(), |_| None))
    ))
);

struct When {
    minute: [bool; MINUTE_COUNT],
    hour: [bool; HOUR_COUNT],
    day_of_month: [bool; DAY_OF_MONTH_COUNT],
    month: [bool; MONTH_COUNT],
    day_of_week: [bool; DAY_OF_WEEK_COUNT],
    minute_star: bool,
    hour_star: bool,
    day_of_month_star: bool,
    day_of_week_star: bool,
}

enum Schedule {
    Reboot,
    When,
}

struct Entry {
    envp: Vec<String>,
    cmd: String,
    schedule: Schedule,
}

pub struct Db {
    entries: Vec<Entry>,
}

impl Db {
    pub fn new() -> Self {
        Db {
            entries: Vec::new(),
        }
    }

    pub fn load(&mut self) -> Result<(), Error> {
        debug!("loading database");
        let system_crontab = File::open("/etc/crontab")?;
        let mut contents = String::new();
        system_crontab.read_to_string(&mut contents)?;
        Ok(())
    }
}
