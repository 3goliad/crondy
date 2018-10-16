use std::str::FromStr;

use nom::{
    alt, call, char, complete, count, digit1, error_position, map, map_opt, map_res, named, opt,
    preceded, sep, separated_list, separated_list_complete, separated_pair, tag, tuple,
    tuple_parser, wrap_sep, ws,
};

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

#[derive(Debug, Clone, PartialEq)]
enum Field {
    Value(usize),
    Range(usize, usize, Option<usize>), // begin, end, step
    List(Vec<(usize, Option<usize>)>),
    Star(Option<usize>), // step
}

named!(integer<&str, usize>, map_res!(digit1, |s| usize::from_str(s)));

named!(step<&str, Option<usize>>, opt!(preceded!(char!('/'), integer)));

named!(parse_range<&str, Field>, complete!(map!(
    tuple!(separated_pair!(integer, char!('-'), integer), step),
    |((begin, end), step)| Field::Range(begin, end, step)
)));

named!(parse_field<&str, Field>, alt!(
    map!(preceded!(char!('*'), step), |opt| Field::Star(opt)) |
    map_opt!(
        separated_list_complete!(
            char!(','),
            alt!(
                map_opt!(parse_range, |f| {
                    match f {
                        Field::Range(begin, end, None) => Some((begin, Some(end))),
                        _ => None
                    }
                }) |
                map!(integer, |i| (i, None))
            )
        ),
        |v: Vec<(usize, Option<usize>)>| {
            if v.len() > 1 {
                Some(Field::List(v))
            } else {
                None
            }
        }) |
    parse_range |
    map!(integer, |i| Field::Value(i))
));

#[cfg(test)]
mod test_parse_field {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn single_char_value() {
        assert_parses_to!(parse_field("9 "), Field::Value(9), " ")
    }
    #[test]
    fn multi_char_value() {
        assert_parses_to!(parse_field("1234 "), Field::Value(1234), " ")
    }

    #[test]
    fn plain_star() {
        assert_parses_to!(parse_field("* "), Field::Star(None), " ")
    }

    #[test]
    fn step_star() {
        assert_parses_to!(parse_field("*/78 "), Field::Star(Some(78)), " ")
    }

    #[test]
    fn range() {
        assert_parses_to!(parse_field("4-5 "), Field::Range(4, 5, None), " ")
    }

    #[test]
    fn range_with_step() {
        assert_parses_to!(parse_field("23-5/40 "), Field::Range(23, 5, Some(40)), " ")
    }

    #[test]
    fn list() {
        assert_parses_to!(
            parse_field("1,2,3 "),
            Field::List(vec![(1, None), (2, None), (3, None)]),
            " "
        )
    }

    #[test]
    fn range_list() {
        assert_parses_to!(
            parse_field("1-4,2-5,3-6,7 "),
            Field::List(vec![(1, Some(4)), (2, Some(5)), (3, Some(6)), (7, None)]),
            " "
        )
    }
}

#[derive(Debug, PartialEq)]
struct When {
    minute: Field,
    hour: Field,
    day_of_month: Field,
    month: Field,
    day_of_week: Field,
}

named!(parse_when<&str, When>, map!(ws!(count!(parse_field, 5)), |fields| {
    When {
        minute: fields[0].clone(),
        hour: fields[1].clone(),
        day_of_month: fields[2].clone(),
        month: fields[3].clone(),
        day_of_week: fields[4].clone()
    }
}));

#[derive(Debug, PartialEq)]
pub enum Schedule {
    Reboot,
    When(When),
}

impl Schedule {
    pub fn parse(input: &str) -> nom::IResult<&str, Self> {
        parse_schedule(input)
    }
}

named!(parse_schedule<&str, Schedule>, alt!(
    map!(tag!("@reboot"),
         |_| Schedule::Reboot) |
    map!(alt!(tag!("@yearly") | tag!("@annually")),
         |_| Schedule::When(When {
             minute: Field::Value(0),
             hour: Field::Value(0),
             day_of_month: Field::Value(1),
             month: Field::Value(1),
             day_of_week: Field::Star(None),
         })) |
    map!(tag!("@monthly"),
         |_| Schedule::When(When {
             minute: Field::Value(0),
             hour: Field::Value(0),
             day_of_month: Field::Value(1),
             month: Field::Star(None),
             day_of_week: Field::Star(None),
         })) |
    map!(tag!("@weekly"),
         |_| Schedule::When(When {
             minute: Field::Value(0),
             hour: Field::Value(0),
             day_of_month: Field::Star(None),
             month: Field::Star(None),
             day_of_week: Field::Value(0),
         })) |
    map!(alt!(tag!("@daily") | tag!("@midnight")),
         |_| Schedule::When(When {
             minute: Field::Value(0),
             hour: Field::Value(0),
             day_of_month: Field::Star(None),
             month: Field::Star(None),
             day_of_week: Field::Star(None),
         })) |
    map!(tag!("@hourly"),
         |_| Schedule::When(When {
             minute: Field::Value(0),
             hour: Field::Star(None),
             day_of_month: Field::Star(None),
             month: Field::Star(None),
             day_of_week: Field::Star(None),
         })) |
    map!(parse_when, |when| Schedule::When(when))
));
