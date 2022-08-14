use std::ops::Range;

use chumsky::{prelude::*, combinator::DelimitedBy, Stream};

use crate::{object::Object, instruction::{Instruction, Dest, Size, Src, Count}};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Token {
    KWFFIRet, 
    KWCopy, 
    KWJif, KWJmp,

    KWFFIBegin,  // spelled "ffinyeh"
    KWFFICall,

    LParen, RParen, LBrack, RBrack,
    Arrow, Minus, Plus, Colon, Comma, Dot, Underscore,
    Number(u64),
    Identifier(String),
}

enum Sign { Minus, Plus }

type Span = Range<usize>;
type Spanned<T> = (T, Span);

pub fn parse(code: &str) -> Result<Spanned<Object>, String> {
    let (tokens, errs) = lexer().then_ignore(end()).parse_recovery(code);
    let (object, parse_errs) = if let Some(tokens) = tokens {
        let len = code.chars().count(); // TODO: What's this bit do?
        parser().then_ignore(end()).parse_recovery(Stream::from_iter(len..len + 1, tokens.into_iter()))

    } else {
        (None, vec![])
    };

    if errs.len() > 0 { return Err(format!("lexer error: {:?}", errs)); }
    if parse_errs.len() > 0 { return Err(format!("parser error: {:?}", parse_errs)); }
    if let Some(obj) = object {
        return Ok(obj)
    }
    // TODO: Report errors
    return Err(format!("no object??? weird"));
}

fn lexer() -> impl Parser<char, Vec<Spanned<Token>>, Error=Simple<char>> + Clone {
    let hex_number = 
        just("0x").ignore_then(
            text::int(16).padded()
            .map(|s: String| u64::from_str_radix(&s, 16).unwrap())
        );

    let decimal_number = 
        text::int(10).padded()
        .map(|s: String| s.parse::<u64>().unwrap());

    let number = hex_number.or(decimal_number);

    let comment = just("%").then(take_until(just('\n'))).padded();

    return choice((
        just("ffinyeh").map(|_| Token::KWFFIBegin),
        just("ffiret").map(|_| Token::KWFFIRet),
        just("copy").map(|_| Token::KWCopy),
        just("jif").map(|_| Token::KWJif),
        just("jmp").map(|_| Token::KWJmp),
        just("fficall").map(|_| Token::KWFFICall),
        just("(").map(|_| Token::LParen),
        just(")").map(|_| Token::RParen),
        just("[").map(|_| Token::LBrack),
        just("]").map(|_| Token::RBrack),
        just("->").map(|_| Token::Arrow),
        just("-").map(|_| Token::Minus),
        just("+").map(|_| Token::Plus),
        just(":").map(|_| Token::Colon),
        just(",").map(|_| Token::Comma),
        just(".").map(|_| Token::Dot),
        just("_").map(|_| Token::Underscore),
        number.map(|i| Token::Number(i)),
        text::ident().map(|s: String| Token::Identifier(s)),
    ))
        .map_with_span(|tok, span| (tok, span))
        .padded_by(comment.repeated())
        .padded()
        .repeated()
}

fn parser() -> impl Parser<Token, Spanned<Object>, Error=Simple<Token>> + Clone {
    let signed_number = 
        choice((just(Token::Minus), just(Token::Plus))).or_not()
        .then(select! { Token::Number(u) => u })
        .map(
            |(sign, num)| 
            (match sign { 
                Some(Token::Minus) => Sign::Minus,
                _ => Sign::Plus
            }, num)
        );

    let dest = just(Token::Underscore).map(|_| Dest::Nowhere);

    let ffi_begin = just(Token::KWFFIBegin).ignore_then(signed_number).then(
        dest
        .separated_by(just(Token::Comma))
        .delimited_by(just(Token::LParen), just(Token::RParen))
    ).then_ignore(just(Token::Dot))
    .try_map(|((sign, n_bytes), destinations), span: Span| {
        if let Sign::Minus = sign { return Err(Simple::custom(span, "can't have a negative-sized frame")) };
        if destinations.len() > 6 { return Err(Simple::custom(span, "can't have > 6 destinations"))}
        let mut real_destinations = [Dest::Nowhere; 6];
        real_destinations[..destinations.len()].clone_from_slice(&destinations);
        Ok(Instruction::FFIBegin(n_bytes, real_destinations))
    });

    /*
    let instruction = choice((
        // FFIBegin
        ffi_begin,
        ffi_begin,
    )).map_with_span(|instruction, span| (instruction, span));
    
    instruction.repeated()
    */
    ffi_begin.map_with_span(|i, s| (i, s)).repeated()
    .map(|instructions| Object { instructions: instructions.into_iter().map(|(x, span)| x).collect() })
    .map_with_span(|o, s| (o, s))
}