use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::alphanumeric1,
    combinator::{complete, map, opt},
    error::{context, ContextError, ParseError, VerboseError},
    multi::many0,
    sequence::{delimited, preceded},
    Finish, IResult,
};

use tracing::debug;

use crate::json_parser::JsonValue;

#[derive(Debug, PartialEq, Eq)]
pub enum Filter {
    FieldAccessor { fields: Vec<String> },
    // Pipeline(Box<Filter>, Box<Filter>),
}

impl Filter {
    pub fn apply<'a>(&self, val: &'a JsonValue) -> Option<&'a JsonValue> {
        debug!("applying {:?} to {:?}", self, val);
        match self {
            Filter::FieldAccessor { fields } => {
                let mut cur = val;
                for field in fields.iter() {
                    cur = cur.as_object()?.get(field)?
                }
                Some(cur)
            }
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
    context("field_accessor_chain", many0(field_accessor))(i)
}

fn root<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Filter, E> {
    delimited(
        opt(sp),
        alt((map(field_accessor_chain, |v| Filter::FieldAccessor {
            fields: v.into_iter().map(|s| s.to_owned()).collect(),
        }),)),
        opt(sp),
    )(i)
}

pub fn parse_filter<'a>(i: &'a str) -> Result<Filter, VerboseError<&'a str>> {
    let filter = complete::<_, _, VerboseError<&'a str>, _>(root)(i)
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
                Filter::FieldAccessor {
                    fields: vec!["hello".into(), "man".into()],
                },
            ),
            (
                ".hi",
                Filter::FieldAccessor {
                    fields: vec!["hi".into()],
                },
            ),
            (".", Filter::FieldAccessor { fields: vec![] }),
        ];

        for (input, output) in cases {
            let res = parse_filter(input);

            if let Err(e) = &res {
                debug!("errors:\n{}", convert_error(input, e.clone()));
            }

            let filter = res.expect("no error");
            assert_eq!(filter, output);
        }
        Ok(())
    }
}
