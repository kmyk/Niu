use nom::IResult;
use nom::character::complete::*;
use nom::sequence::*;
use nom::branch::*;

use crate::literal::{ Literal, parse_literal };
use crate::identifier::{ Identifier, parse_identifier };
use crate::expression::{ Expression, parse_expression };
use crate::subseq::{ Subseq, parse_subseq, subseq_gen_type, subseq_transpile };
use crate::block::{ parse_block, Block };
use crate::structs::*;
use crate::unify::*;
use crate::trans::*;
use crate::type_spec::*;
use crate::traits::*;

#[derive(Debug)]
pub enum UnaryExpr {
    Variable(Variable),
    Literal(Literal),
    Parentheses(Parentheses),
    Block(Block),
    Subseq(Box<UnaryExpr>, Subseq),
    StructInst(StructInstantiation),
    TraitMethod(TypeSpec, TraitMethod),
}

impl GenType for UnaryExpr {
    fn gen_type(&self, equs: &mut TypeEquations) -> TResult {
        match *self {
            UnaryExpr::Variable(ref v) => v.gen_type(equs),
            UnaryExpr::Literal(ref l) => l.gen_type(equs),
            UnaryExpr::Parentheses(ref p) => p.gen_type(equs),
            UnaryExpr::Block(ref b) => b.gen_type(equs),
            UnaryExpr::Subseq(ref expr, ref s) => subseq_gen_type(expr.as_ref(), s, equs),
            UnaryExpr::StructInst(ref inst) => inst.gen_type(equs),
            UnaryExpr::TraitMethod(ref spec, ref tr_id) => Ok(Type::TraitMethod(Box::new(spec.gen_type(equs)?), tr_id.clone())),
        }
    }
}

impl Transpile for UnaryExpr {
    fn transpile(&self, ta: &mut TypeAnnotation) -> String {
        match *self {
            UnaryExpr::Variable(ref v) => v.transpile(ta),
            UnaryExpr::Literal(ref l) => l.transpile(ta),
            UnaryExpr::Parentheses(ref p) => p.transpile(ta),
            UnaryExpr::Block(ref b) => format!("[&](){{ {} }}()", b.transpile(ta)),
            UnaryExpr::Subseq(ref expr, ref s) => subseq_transpile(expr.as_ref(), s, ta),
            UnaryExpr::StructInst(ref inst) => format!("!!!instantiation is unimplemented!!!"),
            UnaryExpr::TraitMethod(ref spec, TraitMethod { ref trait_id, ref method_id }) => {
                format!("{}<{}>::{}", trait_id.transpile(ta), spec.transpile(ta), method_id.transpile(ta))
            }
        }
    }
}

pub fn parse_unary_expr(s: &str) -> IResult<&str, UnaryExpr> {
    let (s, x) = alt((
            parse_unary_trait_method,
            parse_struct_instantiation,
            parse_literal,
            parse_parentheses,
            parse_bracket_block,
            parse_variable,
            ))(s)?;
    let mut now = s;
    let mut prec = x;
    while let Ok((s, sub)) = parse_subseq(now) {
        now = s;
        prec = UnaryExpr::Subseq(Box::new(prec), sub);
    }
    Ok((now, prec))
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Variable {
    pub name: Identifier,
}

impl Variable {
    pub fn from_identifier(id: Identifier) -> Self {
        Variable { name: id }
    }
}

impl GenType for Variable {
    fn gen_type(&self, equs: &mut TypeEquations) -> TResult {
        equs.get_type_from_variable(self)
    }
}

impl Transpile for Variable {
    fn transpile(&self, ta: &mut TypeAnnotation) -> String {
        ta.trans_variable(self)
    }
}

pub fn parse_variable(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, name) = parse_identifier(s)?;
    Ok((s, UnaryExpr::Variable(Variable { name })))
}

#[derive(Debug)]
pub struct Parentheses {
    pub expr: Expression,
}

impl GenType for Parentheses {
    fn gen_type(&self, equs: &mut TypeEquations) -> TResult {
        self.expr.gen_type(equs)
    }
}

impl Transpile for Parentheses {
    fn transpile(&self, ta: &mut TypeAnnotation) -> String {
        format!("({})", self.expr.transpile(ta))
    }
}

pub fn parse_parentheses(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, (_, _, expr, _, _)) = tuple((char('('), space0, parse_expression, space0, char(')')))(s)?;
    Ok((s, UnaryExpr::Parentheses(Parentheses { expr })))
}

pub fn parse_bracket_block(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, (_, _, block, _, _)) = tuple((char('{'), space0, parse_block, space0, char('}')))(s)?;
    Ok((s, UnaryExpr::Block(block)))
}

pub fn parse_unary_trait_method(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, (spec, _, _, _, trait_method)) = tuple((parse_type_spec, space0, char('#'), space0, parse_trait_method))(s)?;
    Ok((s, UnaryExpr::TraitMethod(spec, trait_method)))
}

#[test]
fn parse_unary_expr_test() {
    println!("{:?}", parse_unary_expr("func(1, 2, 3)"));
    println!("{:?}", parse_unary_expr("add(1, add(2, 3), 4)"));
    println!("{:?}", parse_unary_expr("generate_func(91)(1333)"));
    println!("{:?}", parse_unary_expr("MyStruct { a: 1i64 + 2i64, b: val, }"));
    println!("{:?}", parse_unary_expr("generate_func(31 * 91, 210)(1333 / 5 * 3)"));
}
#[test]
fn parse_parentheses_expr_test() {
    println!("{:?}", parse_unary_expr("(1 + 2 + 3)"));
}
#[test]
fn parse_trait_method_test() {
    println!("{:?}", parse_unary_expr("i64#MyTrait.out"));
}
