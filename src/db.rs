use log::debug;
use std::fs::File;
use std::io::{Error, Read};

use crate::schedule::Schedule;
use nom::{
    alt, alt_complete, call, char, complete, eof, error_position, exact, many1, map, named,
    none_of, preceded, rest, separated_pair, space0, terminated, tuple, tuple_parser,
};

#[derive(Debug, PartialEq)]
enum CrontabLine {
    Entry(Entry),
    Env(String, String),
}

named!(parse_command<&str, String>, map!(rest, |s| s.to_owned()));

named!(parse_entry<&str, Option<CrontabLine>>, map!(
    tuple!(Schedule::parse, parse_command),
    |(schedule, cmd)| Some(CrontabLine::Entry(
        Entry {
            envp: Vec::new(),
            cmd,
            schedule,
        }
    ))));

named!(
    parse_env<&str, CrontabLine>,
    map!(
        separated_pair!(many1!(none_of!("=")), char!('='), rest),
        |(n, v)| {
            let name = n.into_iter().collect::<String>();
            let mut value = v.trim_left();
            if (value.starts_with('\'') && value.ends_with('\'')) || (value.starts_with('"') && value.ends_with('"')) {
                value = &value[1..(value.len() - 1)];
            }
            CrontabLine::Env(name.trim_right().to_string(), value.to_string())
        }
    )
);

#[cfg(test)]
mod test_parse_env {
    use super::*;

    #[test]
    fn parses_close_pairs() {
        assert_parses_to_exactly!(
            parse_env("FOO=bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn parses_distant_pairs() {
        assert_parses_to_exactly!(
            parse_env("FOO = bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn strips_trailing_whitespace_from_name() {
        assert_parses_to_exactly!(
            parse_env("FOO =bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn strips_leading_whitespace_from_value() {
        assert_parses_to_exactly!(
            parse_env("FOO= bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn preserves_trailing_whitespace_in_value() {
        assert_parses_to_exactly!(
            parse_env("FOO=bar "),
            CrontabLine::Env("FOO".to_string(), "bar ".to_string())
        )
    }

    #[test]
    fn preserves_spaces_in_value() {
        assert_parses_to_exactly!(
            parse_env("FOO=bar baz"),
            CrontabLine::Env("FOO".to_string(), "bar baz".to_string())
        )
    }

    #[test]
    fn preserves_spaces_in_single_quotes() {
        assert_parses_to_exactly!(
            parse_env("FOO=' baz'"),
            CrontabLine::Env("FOO".to_string(), " baz".to_string())
        )
    }

    #[test]
    fn preserves_spaces_in_double_quotes() {
        assert_parses_to_exactly!(
            parse_env("FOO=\" baz\""),
            CrontabLine::Env("FOO".to_string(), " baz".to_string())
        )
    }

    #[test]
    fn preserves_spaces_in_name() {
        assert_parses_to_exactly!(
            parse_env("FOO BAR=baz"),
            CrontabLine::Env("FOO BAR".to_string(), "baz".to_string())
        )
    }
}

named!(parse_comment<&str, &str>, preceded!(char!('#'), rest));

named!(
    parse_line<&str, Option<CrontabLine>>,
    exact!(preceded!(
        space0,
        alt_complete!(parse_entry | map!(parse_env, |l| Some(l)) | map!(parse_comment, |_| None) | map!(eof!(), |_| None))
    ))
);

#[derive(Debug, PartialEq)]
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
        let mut system_crontab = File::open("/etc/crontab")?;
        let mut contents = String::new();
        system_crontab.read_to_string(&mut contents)?;
        Ok(())
    }
}
