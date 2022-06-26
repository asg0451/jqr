use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::alphanumeric1,
    combinator::{all_consuming, map, opt},
    error::{context, ContextError, ParseError, VerboseError},
    multi::{many1, separated_list0, separated_list1},
    sequence::{delimited, preceded, tuple},
    Finish, IResult,
};

use tracing::debug;

use crate::json_parser::JsonValue;

#[derive(Debug, PartialEq, Eq)]
pub struct Pipeline {
    filters: Vec<Filter>,
}

impl Pipeline {
    pub fn apply<'a>(&self, val: &'a JsonValue) -> Option<JsonValue> {
        self.filters
            .iter()
            .fold(Some(val.clone()), |acc, f| f.apply(&acc?))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Filter {
    FieldAccessor { fields: Vec<String> },
    FunctionCall { name: String, args: Vec<String> },
    // Spread {  }
}

impl Filter {
    // TODO: made this clone everything because the Cows were killing me
    // the FieldAccessor branch can return a reference (how?), but the function call branch creates data

    pub fn apply<'a>(&self, val: &'a JsonValue) -> Option<JsonValue> {
        debug!("applying {:?} to {:?}", self, val);
        match self {
            Filter::FieldAccessor { fields } => {
                let mut cur = val;
                for field in fields.iter() {
                    cur = cur.as_object()?.get(field)?
                }
                Some(cur.clone())
            }
            // TODO: improve this
            Filter::FunctionCall { name, args } => match name.as_str() {
                "length" => match val {
                    // TODO: unsafe "as"
                    JsonValue::Array(a) => Some(JsonValue::Num(a.len() as f64)),
                    JsonValue::Object(o) => Some(JsonValue::Num(o.len() as f64)),
                    _ => panic!("cannot take length of non-array/object"),
                },
                "split" => {
                    let delim = &args[0];
                    match val {
                        JsonValue::Str(s) => Some(JsonValue::Array(
                            s.split(delim)
                                .map(|e| JsonValue::Str(e.to_string()))
                                .collect::<Vec<_>>(),
                        )),
                        _ => panic!("cannot split non-string"),
                    }
                }
                _ => panic!("unknown function {}", name),
            },
        }
    }
}

fn sp<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    take_while(move |c| "\t ".contains(c))(i)
}

fn identifier<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context("identifier", alphanumeric1)(i)
}

fn field_accessor<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context("field_accessor", preceded(tag("."), identifier))(i)
}

fn field_accessor_chain<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<&'a str>, E> {
    context("field_accessor_chain", many1(field_accessor))(i)
}

fn function_name<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context("function_name", identifier)(i)
}

// TODO: should be able to / have to put args in "quotes" unless numeric idk lol. maybe arg is a JsonValue?
// but also the filter itself should be able to be a JsonValue so idk
fn function_arg<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context("function_arg", identifier)(i)
}

fn function_args<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<&'a str>, E> {
    context(
        "function_args",
        separated_list0(delimited(opt(sp), tag(","), opt(sp)), function_arg),
    )(i)
}

// TODO: "()" should be optional
fn function_call<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, (&'a str, Vec<&'a str>), E> {
    context(
        "function_call",
        tuple((function_name, delimited(tag("("), function_args, tag(")")))),
    )(i)
}

fn filter<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Filter, E> {
    delimited(
        opt(sp),
        alt((
            // special case: '.'
            map(all_consuming(tag(".")), |_| Filter::FieldAccessor {
                fields: vec![],
            }),
            map(field_accessor_chain, |v| Filter::FieldAccessor {
                fields: v.into_iter().map(|s| s.to_owned()).collect(),
            }),
            map(function_call, |(name, args)| Filter::FunctionCall {
                name: name.to_owned(),
                args: args.into_iter().map(|s| s.to_owned()).collect(),
            }),
        )),
        opt(sp),
    )(i)
}

fn pipeline<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<Filter>, E> {
    context(
        "pipeline",
        separated_list1(delimited(opt(sp), tag("|"), opt(sp)), filter),
    )(i)
}

fn root<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Pipeline, E> {
    delimited(
        opt(sp),
        map(pipeline, |filters| Pipeline { filters }),
        opt(sp),
    )(i)
}

pub fn parse_filter<'a>(i: &'a str) -> Result<Pipeline, VerboseError<&'a str>> {
    let filter = all_consuming::<_, _, VerboseError<&'a str>, _>(root)(i)
        .finish()?
        .1;
    Ok(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use nom::error::convert_error;

    #[test]
    fn it_works() -> Result<()> {
        let cases = [
            (
                " .hello.man ",
                Pipeline {
                    filters: vec![Filter::FieldAccessor {
                        fields: vec!["hello".into(), "man".into()],
                    }],
                },
            ),
            (
                ".hi",
                Pipeline {
                    filters: vec![Filter::FieldAccessor {
                        fields: vec!["hi".into()],
                    }],
                },
            ),
            (
                ".",
                Pipeline {
                    filters: vec![Filter::FieldAccessor { fields: vec![] }],
                },
            ),
            (
                ".a | .b",
                Pipeline {
                    filters: vec![
                        Filter::FieldAccessor {
                            fields: vec!["a".into()],
                        },
                        Filter::FieldAccessor {
                            fields: vec!["b".into()],
                        },
                    ],
                },
            ),
            (
                "hello(42)",
                Pipeline {
                    filters: vec![Filter::FunctionCall {
                        name: "hello".into(),
                        args: vec!["42".into()],
                    }],
                },
            ),
            (
                ".a | hello(42)",
                Pipeline {
                    filters: vec![
                        Filter::FieldAccessor {
                            fields: vec!["a".into()],
                        },
                        Filter::FunctionCall {
                            name: "hello".into(),
                            args: vec!["42".into()],
                        },
                    ],
                },
            ),
            // TODO: why this one no worky
            (
                ". | hello(42)",
                Pipeline {
                    filters: vec![
                        Filter::FieldAccessor { fields: vec![] },
                        Filter::FunctionCall {
                            name: "hello".into(),
                            args: vec!["42".into()],
                        },
                    ],
                },
            ),
        ];

        for (input, output) in cases {
            let res = parse_filter(input);

            if let Err(e) = &res {
                eprintln!("errors:\n{}", convert_error(input, e.clone()));
            }

            let filter = res.expect("no error");
            assert_eq!(filter, output);
        }
        Ok(())
    }
}
