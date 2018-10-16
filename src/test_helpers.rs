macro_rules! assert_parses_to {
    ($parse:expr, $to:expr, $rest:expr) => {{
        let iresult = $parse;
        let (remaining, result) = iresult.unwrap();
        assert_eq!(result, $to);
        assert_eq!(remaining, $rest);
    }};
}

macro_rules! assert_parses_to_exactly {
    ($parse:expr, $to:expr) => {{
        assert_parses_to!($parse, $to, "");
    }};
}
