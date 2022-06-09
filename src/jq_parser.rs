use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::{
        streaming::alphanumeric1,
    },
    combinator::{all_consuming, map, opt},
    error::{context, ContextError, ParseError, VerboseError},
    multi::{many1},
    sequence::{delimited, preceded}, Finish, IResult,
};

use std::str;

use crate::json_parser::JsonValue;

#[derive(Debug)]
pub enum Filter {
    FieldAccessor { fields: Vec<String> },
    // Pipeline(Box<Filter>, Box<Filter>),
}

impl Filter {
    pub fn apply<'a>(&self, val: &'a JsonValue) -> Option<&'a JsonValue> {
        eprintln!("applying {:?} to {:?}", self, val);
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
    context("field_accessor_chain", many1(field_accessor))(i)
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
    let filter = all_consuming::<_, _, VerboseError<&str>, _>(root)(i)
        .finish()?
        .1;
    Ok(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn it_works() -> Result<()> {
        // TODO: this one also has issues if there arent trailing spaces or something... why
        let input = " .hello.man ";
        let res = root::<VerboseError<&str>>(input);
        dbg!(res);

        // if let Err(e) = res {
        //     eprintln!("errors:\n{}", convert_error(input, e));
        // }

        Ok(())
    }
}
