use nom::branch::*;
use nom::IResult;
use nom::character::complete::*;
use nom::bytes::complete::*;
use nom::sequence::*;

use crate::expression::{ Expression, parse_expression };
use crate::let_declaration::{ LetDeclaration, parse_let_declaration };

#[derive(Debug)]
pub enum Statement<'a> {
    Expression(Expression<'a>),
    LetDeclaration(LetDeclaration<'a>),
}

fn parse_expression_to_statement(s: &str) -> IResult<&str, Statement> {
    let (s, expr) = parse_expression(s)?;
    Ok((s, Statement::Expression(expr)))
}

fn parse_let_declaration_to_statement(s: &str) -> IResult<&str, Statement> {
    let (s, decl) = parse_let_declaration(s)?;
    Ok((s, Statement::LetDeclaration(decl)))
}

pub fn parse_statement(s: &str) -> IResult<&str, Statement> {
    let (s, (_, statement, _, _semi)) =
        tuple((space0, alt((parse_let_declaration_to_statement, parse_expression_to_statement)), space0, tag(";")))(s)?;
    Ok((s, statement))
}

#[test]
fn parse_statement_test() {
    println!("{:?}", parse_statement("let x = 1 + 2;"));
    println!("{:?}", parse_statement("clamp(x, y, z);"));
}
