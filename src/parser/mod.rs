use nom::{bytes::complete::tag};

use crate::{object::Object, instruction::{Instruction, Dest, Size, Src, Count}};

pub fn parse_object(input: &str) -> nom::IResult<&str, Object> {
    nom::combinator::all_consuming(parse_object_internal)(input)
}
fn parse_object_internal(input: &str) -> nom::IResult<&str, Object> {
    let (input, instructions) = ws(nom::multi::many0(parse_instruction))(input)?;

    Ok((input, Object { instructions }))
}

fn parse_instruction(input: &str) -> nom::IResult<&str, Instruction> {
    nom::branch::alt((
        parse_begin,
        parse_ret,
        parse_copy
    ))(input)
}

fn parse_begin(input: &str) -> nom::IResult<&str, Instruction> {
    // nyeh 180 ~ (bp-4 BYTE, [bp-8] BYTE, [bp-12]->4 DWORD).
    let (input, _) = ws(tag("nyeh"))(input)?;
    let (input, n_bytes) = ws(nom::character::complete::u64)(input)?;
    let (input, mut args) = ws(nom::sequence::delimited(
        tag("("), 
        nom::multi::separated_list0(
            parse_argsep, parse_dest
        ),
        tag(")")
    ))(input)?;
    let (input, _) = ws(parse_lineterm)(input)?;

    while args.len() < 6 {
        args.push(Dest::Nowhere)
    }

    if args.len() > 6 { return Err(nom::Err::Error(nom::error::make_error(input, nom::error::ErrorKind::Fail))) }

    let args: [Dest; 6] = args.try_into().expect("must have exactly 6 args");

    Ok((input, Instruction::Begin(n_bytes, args)))
}

fn parse_ret(input: &str) -> nom::IResult<&str, Instruction> {
    // ret bp-4.
    let (input, _) = ws(tag("ret"))(input)?;
    let (input, src) = parse_src(input)?;
    let (input, _) = parse_lineterm(input)?;

    Ok((input, Instruction::Ret(src)))
}

fn parse_copy(input: &str) -> nom::IResult<&str, Instruction> {
    println!("Here: {}", input);
    // bp-4 = bp-8 (4x).
    let (input, dest) = parse_dest(input)?;
    let (input, _) = ws(tag("="))(input)?;
    let (input, src) = parse_src(input)?;
    let (input, count) = nom::combinator::opt(parse_count)(input)?;
    let (input, _) = parse_lineterm(input)?;
    
    Ok((input, Instruction::Copy(dest, src, count.unwrap_or(Count(1)))))
}

fn parse_count(input: &str) -> nom::IResult<&str, Count> {
    let (input, _) = ws(tag("("))(input)?;
    let (input, n) = ws(nom::sequence::delimited(
        nom::combinator::success( () ), 
        nom::character::complete::u64,
        tag("x"),
    ))(input)?;
    let (input, _) = ws(tag(")"))(input)?;
    Ok((input, Count(n)))
}

fn parse_dest(input: &str) -> nom::IResult<&str, Dest> {
    nom::branch::alt((parse_dest_nowhere, parse_dest_ptr, parse_dest_here))(input)
}

fn parse_dest_nowhere(input: &str) -> nom::IResult<&str, Dest> {
    let (input, _) = ws(tag("_"))(input)?;
    Ok((input, Dest::Nowhere))
}

fn parse_dest_ptr(input: &str) -> nom::IResult<&str, Dest> {
    let (input, _) = ws(tag("["))(input)?;
    let (input, offset_to_ptr) = parse_here_expr(input)?;
    let (input, _) = ws(tag("]"))(input)?;
    let (input, offset_after_ptr) = nom::combinator::opt(parse_field_tag)(input)?;
    let (input, size) = parse_size(input)?;

    Ok((input, Dest::Ptr(offset_to_ptr, offset_after_ptr.unwrap_or(0), size)))
}

fn parse_dest_here(input: &str) -> nom::IResult<&str, Dest> {
    let (input, stack_position) = parse_here_expr(input)?;
    let (input, size) = parse_size(input)?;

    Ok((input, Dest::Here(stack_position, size)))
}

fn parse_src(input: &str) -> nom::IResult<&str, Src> {
    nom::branch::alt((parse_src_imm, parse_src_ptr, parse_src_here))(input)
}

fn parse_src_imm(input: &str) -> nom::IResult<&str, Src> {
    let (input, immediate) = ws(nom::character::complete::u64)(input)?;
    return Ok((input, Src::Imm(immediate)))
}

fn parse_src_ptr(input: &str) -> nom::IResult<&str, Src> {
    let (input, _) = ws(tag("["))(input)?;
    let (input, offset_to_ptr) = parse_here_expr(input)?;
    let (input, _) = ws(tag("]"))(input)?;
    let (input, offset_after_ptr) = nom::combinator::opt(parse_field_tag)(input)?;
    let (input, size) = parse_size(input)?;

    Ok((input, Src::Ptr(offset_to_ptr, offset_after_ptr.unwrap_or(0), size)))
}

fn parse_src_here(input: &str) -> nom::IResult<&str, Src> {
    let (input, stack_position) = parse_here_expr(input)?;
    let (input, size) = parse_size(input)?;

    Ok((input, Src::Here(stack_position, size)))
}

fn parse_here_expr(input: &str) -> nom::IResult<&str, i32>{
    let (input, _) = ws(tag("bp"))(input)?;
    let (input, sign) = ws(nom::branch::alt((tag("-"), tag("+"))))(input)?;
    let (input, amt) = ws(nom::character::complete::i32)(input)?;

    Ok((input, if sign == "-" {
        -amt
    } else {
        amt
    }))
}

fn parse_field_tag(input: &str) -> nom::IResult<&str, i32> {
    let (input, _) = ws(tag("->"))(input)?;
    let (input, offset) = ws(nom::character::complete::i32)(input)?;
    Ok((input, offset))
}

fn parse_size(input: &str) -> nom::IResult<&str, Size> {
    let (input, result) = ws(nom::branch::alt((
        nom::combinator::value(Size::B, tag("B")),
        nom::combinator::value(Size::H, tag("H")),
        nom::combinator::value(Size::D, tag("D")),
        nom::combinator::value(Size::Q, tag("Q")),
    )))(input)?;

    Ok((input, result))
}

fn parse_lineterm(input: &str) -> nom::IResult<&str, ()> {
    let (input, _) = ws(tag("."))(input)?;
    Ok((input, ()))
}

fn parse_argsep(input: &str) -> nom::IResult<&str, ()> {
    let (input, _) = ws(tag(","))(input)?;
    Ok((input, ()))
}

// from: https://docs.rs/nom/latest/nom/recipes/index.html
/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and 
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: nom::error::ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> nom::IResult<&'a str, O, E>
  where
  F: FnMut(&'a str) -> nom::IResult<&'a str, O, E>,
{
  nom::sequence::delimited(
    nom::character::complete::multispace0,
    inner,
    nom::character::complete::multispace0
  )
}