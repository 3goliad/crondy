use log::{debug, error};

use crate::schedule::Schedule;
use failure::Error;
use nom::{
    alt, alt_complete, call, char, complete, error_position, many1, map, named, none_of, preceded,
    rest, separated_pair, tuple, tuple_parser, AsChar, InputTakeAtPosition,
};

#[derive(Debug)]
pub struct Crontab {
    entries: Vec<Entry>,
}

impl Crontab {
    pub fn parse(input: &str) -> Result<Self, std::io::Error> {
        let mut entries = Vec::new();
        let mut env = Vec::new();
        for line in input.lines() {
            match parse_line(line) {
                Ok(("", line)) => match line {
                    Some(CrontabLine::Entry(mut e)) => {
                        e.envp = env.clone();
                        entries.push(e);
                    }
                    Some(CrontabLine::Env(n, v)) => env.push(format!("{}={}", n, v)),
                    None => debug!("parsed an empty line"),
                },
                Ok((remaining, _)) => error!("leftovers {}", remaining),
                Err(nom::Err::Incomplete(_)) => error!("incomplete"),
                Err(nom::Err::Error(_)) => error!("errorerrorerror"),
                Err(nom::Err::Failure(_)) => error!("failure"),
            }
        }
        Ok(Self { entries })
    }

    pub fn validate(&self) -> Result<(), Error> {
        for entry in self.entries.iter() {
            entry.schedule.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
enum CrontabLine {
    Entry(Entry),
    Env(String, String),
}

#[derive(Debug, PartialEq)]
struct Entry {
    envp: Vec<String>,
    cmd: String,
    schedule: Schedule,
}

named!(
    parse_line<&str, Option<CrontabLine>>,
    alt_complete!(
        map!(parse_entry, |e| Some(CrontabLine::Entry(e))) |
        map!(parse_env, |l| Some(l)) |
        map!(parse_comment, |_| None) |
        map!(empty_line, |_| None)
    )
);

fn empty_line(input: &str) -> nom::IResult<&str, &str> {
    if input.len() == 0 {
        Ok(("", input))
    } else {
        if input.chars().all(|c| c == ' ' || c == '\t') {
            Ok(("", input))
        } else {
            input.split_at_position(|item| {
                let c = item.clone().as_char();
                !(c == ' ' || c == '\t')
            })
        }
    }
}

named!(parse_entry<&str, Entry>, map!(
    tuple!(Schedule::parse, map!(rest, |s| s.to_owned())),
    |(schedule, cmd)|
        Entry {envp: Vec::new(), cmd, schedule}
    ));

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

named!(parse_comment<&str, &str>, preceded!(char!('#'), rest));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_crontab_blanks() {
        assert!(Crontab::parse("").is_ok())
    }

    #[test]
    fn parse_crontab_empty_lines() {
        assert!(Crontab::parse("  \n  \n").is_ok())
    }

    #[test]
    fn parse_crontab_comment_and_schedule() {
        let tab = Crontab::parse("\n# Howdy pardner\n* * * * * this is a command\n").unwrap();
        assert!(tab.entries.len() == 1)
    }

    #[test]
    fn parse_crontab_applies_env_to_later_schedules() {
        let tab = Crontab::parse("* * * * * first\nFOO = BAR\n* * * * 3 second").unwrap();
        assert_eq!(tab.entries[0].envp, Vec::<String>::new());
        assert_eq!(tab.entries[1].envp, vec!["FOO=BAR".to_owned()]);
    }

    #[test]
    fn parse_line_nothing() {
        assert_parses_to_exactly!(parse_line(""), None)
    }

    #[test]
    fn parse_line_blanks() {
        assert_parses_to_exactly!(parse_line(" \t"), None)
    }

    #[test]
    fn parse_line_stops_at_newline_after_nothing() {
        assert_parses_to!(parse_line("\n"), None, "\n")
    }

    #[test]
    fn parse_line_stops_at_newline_after_blanks() {
        assert_parses_to!(parse_line("  \n"), None, "\n")
    }

    #[test]
    fn parse_entry_all_stars() {
        let (rem, entry) = parse_entry("* * * * * this is a command").unwrap();
        assert_eq!(rem, "");
        assert_eq!(entry.cmd, " this is a command".to_owned());
    }

    #[test]
    fn parse_env_parses_close_pairs() {
        assert_parses_to_exactly!(
            parse_env("FOO=bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn parse_env_parses_distant_pairs() {
        assert_parses_to_exactly!(
            parse_env("FOO = bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn parse_env_strips_trailing_whitespace_from_name() {
        assert_parses_to_exactly!(
            parse_env("FOO =bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn parse_env_strips_leading_whitespace_from_value() {
        assert_parses_to_exactly!(
            parse_env("FOO= bar"),
            CrontabLine::Env("FOO".to_string(), "bar".to_string())
        )
    }

    #[test]
    fn parse_env_preserves_trailing_whitespace_in_value() {
        assert_parses_to_exactly!(
            parse_env("FOO=bar "),
            CrontabLine::Env("FOO".to_string(), "bar ".to_string())
        )
    }

    #[test]
    fn parse_env_preserves_spaces_in_value() {
        assert_parses_to_exactly!(
            parse_env("FOO=bar baz"),
            CrontabLine::Env("FOO".to_string(), "bar baz".to_string())
        )
    }

    #[test]
    fn parse_env_preserves_spaces_in_single_quotes() {
        assert_parses_to_exactly!(
            parse_env("FOO=' baz'"),
            CrontabLine::Env("FOO".to_string(), " baz".to_string())
        )
    }

    #[test]
    fn parse_env_preserves_spaces_in_double_quotes() {
        assert_parses_to_exactly!(
            parse_env("FOO=\" baz\""),
            CrontabLine::Env("FOO".to_string(), " baz".to_string())
        )
    }

    #[test]
    fn parse_env_preserves_spaces_in_name() {
        assert_parses_to_exactly!(
            parse_env("FOO BAR=baz"),
            CrontabLine::Env("FOO BAR".to_string(), "baz".to_string())
        )
    }

    #[test]
    fn parse_comment_hash() {
        assert_parses_to_exactly!(parse_comment("#"), "")
    }

    #[test]
    fn parse_comment_garbage() {
        assert_parses_to_exactly!(parse_comment("# trash here"), " trash here")
    }
}
