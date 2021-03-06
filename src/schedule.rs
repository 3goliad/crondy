use std::str::FromStr;

use nom::{
    alt, call, char, complete, digit1, error_position, map, map_opt, map_res, named, opt, preceded,
    separated_list, separated_list_complete, separated_pair, space1, tag, tuple, tuple_parser,
};

use failure::{bail, format_err, Error};

const FIRST_MINUTE: usize = 0;
const LAST_MINUTE: usize = 59;

const FIRST_HOUR: usize = 0;
const LAST_HOUR: usize = 23;

const FIRST_DAY_OF_MONTH: usize = 1;
const LAST_DAY_OF_MONTH: usize = 31;

const FIRST_MONTH: usize = 1;
const LAST_MONTH: usize = 12;

/* note on DAY_OF_WEEK: 0 and 7 are both Sunday, for compatibility reasons. */
const FIRST_DAY_OF_WEEK: usize = 0;
const LAST_DAY_OF_WEEK: usize = 7;

#[derive(Debug, PartialEq)]
pub enum Schedule {
    Reboot,
    When(When),
}

impl Schedule {
    pub fn parse(input: &str) -> nom::IResult<&str, Self> {
        parse_schedule(input)
    }

    pub fn validate(&self) -> Result<(), Error> {
        match &self {
            Schedule::Reboot => Ok(()),
            Schedule::When(when) => when.validate(),
        }
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

#[derive(Debug, PartialEq)]
pub struct When {
    pub minute: Field,
    pub hour: Field,
    pub day_of_month: Field,
    pub month: Field,
    pub day_of_week: Field,
}

impl When {
    pub fn validate(&self) -> Result<(), Error> {
        self.minute
            .validate(FIRST_MINUTE, LAST_MINUTE)
            .map_err(|e| format_err!("minute {}", e))
            .and(
                self.hour
                    .validate(FIRST_HOUR, LAST_HOUR)
                    .map_err(|e| format_err!("hour {}", e)),
            )
            .and(
                self.day_of_month
                    .validate(FIRST_DAY_OF_MONTH, LAST_DAY_OF_MONTH)
                    .map_err(|e| format_err!("day of month {}", e)),
            )
            .and(
                self.month
                    .validate(FIRST_MONTH, LAST_MONTH)
                    .map_err(|e| format_err!("month {}", e)),
            )
            .and(
                self.day_of_week
                    .validate(FIRST_DAY_OF_WEEK, LAST_DAY_OF_WEEK)
                    .map_err(|e| format_err!("day of week {}", e)),
            )
    }
}

fn parse_when(input: &str) -> nom::IResult<&str, When> {
    named!(inner<&str, Vec<Field>>, separated_list_complete!(space1, parse_field));
    match inner(input) {
        Ok((remaining, fields)) => {
            if fields.len() == 5 {
                Ok((
                    remaining,
                    When {
                        minute: fields[0].clone(),
                        hour: fields[1].clone(),
                        day_of_month: fields[2].clone(),
                        month: fields[3].clone(),
                        day_of_week: fields[4].clone(),
                    },
                ))
            } else {
                Err(nom::Err::Incomplete(nom::Needed::Unknown))
            }
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Value(usize),
    Range(usize, usize, Option<usize>), // begin, end, step
    List(Vec<(usize, Option<usize>)>),
    Star(Option<usize>), // step
}

impl Field {
    pub fn validate(&self, lower_bound: usize, upper_bound: usize) -> Result<(), Error> {
        match self {
            Field::Value(value) => {
                if *value < lower_bound {
                    bail!(
                        "value too low (got {} but expected no less than {})",
                        value,
                        lower_bound
                    );
                }
                if *value > upper_bound {
                    bail!(
                        "value too high (got {} but expected no more than {})",
                        value,
                        upper_bound
                    );
                }
            }
            Field::Range(start, end, maybe_step) => {
                if start > end {
                    bail!(
                        "range out of order (start {} came after end {})",
                        start,
                        end
                    );
                }
                if let Some(step) = maybe_step {
                    if (start + step) >= *end {
                        bail!(
                            "step too big (range only covers {} but step was {})",
                            end - start,
                            step
                        );
                    }
                }
                if *start < lower_bound {
                    bail!(
                        "range start too low (got {} but expected no less than {})",
                        start,
                        lower_bound
                    );
                }
                if *end > upper_bound {
                    bail!(
                        "range end too high (got {} but expected no more than {})",
                        end,
                        upper_bound
                    );
                }
            }
            Field::Star(None) => (),
            Field::Star(Some(step)) => {
                if *step > upper_bound - lower_bound {
                    bail!(
                        "step too big (field only covers {} but step was {})",
                        upper_bound - lower_bound,
                        step
                    );
                }
            }
            Field::List(items) => {
                for item in items {
                    match item {
                        (value, None) => {
                            if *value < lower_bound {
                                bail!(
                                    "list value too low (got {} but expected no less than {})",
                                    value,
                                    lower_bound
                                );
                            }
                            if *value > upper_bound {
                                bail!(
                                    "list value too high (got {} but expected no more than {})",
                                    value,
                                    upper_bound
                                );
                            }
                        }
                        (start, Some(end)) => {
                            if start > end {
                                bail!(
                                    "list range out of order (start {} came after end {})",
                                    start,
                                    end
                                );
                            }
                            if *start < lower_bound {
                                bail!("list range start too low (got {} but expected no less than {})", start, lower_bound);
                            }
                            if *end > upper_bound {
                                bail!(
                                    "list range end too high (got {} but expected no more than {})",
                                    end,
                                    upper_bound
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
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
mod tests {
    use super::*;

    #[test]
    fn parses_all_stars() {
        assert_parses_to!(
            parse_when("* * * * * "),
            When {
                minute: Field::Star(None),
                hour: Field::Star(None),
                day_of_month: Field::Star(None),
                month: Field::Star(None),
                day_of_week: Field::Star(None)
            },
            " "
        )
    }

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
    fn star_with_space() {
        assert_parses_to!(parse_field("* *"), Field::Star(None), " *")
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
