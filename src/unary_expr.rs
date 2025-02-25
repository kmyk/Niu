use nom::IResult;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::sequence::*;
use nom::branch::*;
use nom::combinator::*;
use nom::multi::*;

use crate::literal::{ Literal, parse_literal };
use crate::identifier::{ Identifier, parse_identifier };
use crate::expression::{ Expression, parse_expression };
use crate::subseq::*;
use crate::block::{ parse_block, Block };
use crate::structs::*;
use crate::unify::*;
use crate::trans::*;
use crate::mut_checker::*;
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
    TraitMethod(TypeSpec, Option<TraitSpec>, Identifier),
}

impl GenType for UnaryExpr {
    fn gen_type(&self, equs: &mut TypeEquations, trs: &TraitsInfo) -> TResult {
        match *self {
            UnaryExpr::Variable(ref v) => v.gen_type(equs, trs),
            UnaryExpr::Literal(ref l) => l.gen_type(equs, trs),
            UnaryExpr::Parentheses(ref p) => p.gen_type(equs, trs),
            UnaryExpr::Block(ref b) => b.gen_type(equs, trs),
            UnaryExpr::Subseq(ref expr, ref s) => subseq_gen_type(expr.as_ref(), s, equs, trs),
            UnaryExpr::StructInst(ref inst) => inst.gen_type(equs, trs),
            UnaryExpr::TraitMethod(ref spec, ref trait_spec, ref mem_id) => {
                let alpha = mem_id.generate_type_variable("FuncTypeInfo", 0, equs);
                let trait_gen = match trait_spec {
                    Some(trait_spec) => {
                        Some(trait_spec.generate_trait_generics(equs, trs, &GenericsTypeMap::empty())?)
                    }
                    None => None,
                };
                let right = Type::TraitMethod(Box::new(spec.generics_to_type(&GenericsTypeMap::empty(), equs, trs)?), trait_gen.clone(), mem_id.clone());
                equs.add_equation(alpha, right);
                Ok(Type::TraitMethod(Box::new(spec.generics_to_type(&GenericsTypeMap::empty(), equs, trs)?), trait_gen.clone(), mem_id.clone()))
            }
        }
    }
}

impl Transpile for UnaryExpr {
    fn transpile(&self, ta: &TypeAnnotation) -> String {
        match *self {
            UnaryExpr::Variable(ref v) => v.transpile(ta),
            UnaryExpr::Literal(ref l) => l.transpile(ta),
            UnaryExpr::Parentheses(ref p) => p.transpile(ta),
            UnaryExpr::Block(ref b) => format!("[&](){{ {} }}()", b.transpile(ta)),
            UnaryExpr::Subseq(ref expr, ref s) => subseq_transpile(expr.as_ref(), s, ta),
            UnaryExpr::StructInst(ref inst) => inst.transpile(ta),
            UnaryExpr::TraitMethod(ref spec, Some(ref trait_spec), ref method_id) => {
                format!("{}<{}>::{}", trait_spec.trait_id.transpile(ta), spec.transpile(ta), method_id.into_string())
            }
            UnaryExpr::TraitMethod(ref spec, _, ref method_id) => {
                format!("{}::{}", spec.transpile(ta), method_id.into_string())
            }
        }
    }
}

impl MutCheck for UnaryExpr {
    fn mut_check(&self, ta: &TypeAnnotation, vars: &mut VariablesInfo) -> Result<MutResult, String> {
        match *self {
            UnaryExpr::Variable(ref v) => v.mut_check(ta, vars),
            UnaryExpr::Literal(ref l) => l.mut_check(ta, vars),
            UnaryExpr::Parentheses(ref p) => p.mut_check(ta, vars),
            UnaryExpr::Block(ref b) => b.mut_check(ta, vars),
            UnaryExpr::Subseq(ref expr, ref s) => subseq_mut_check(expr.as_ref(), s, ta, vars),
            UnaryExpr::StructInst(ref inst) => inst.mut_check(ta, vars),
            UnaryExpr::TraitMethod(ref spec, Some(ref trait_id), ref method_id) => {
                Ok(MutResult::NotMut)
            }
            UnaryExpr::TraitMethod(ref spec, _, ref method_id) => {
                Ok(MutResult::NotMut)
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
    pub id: Identifier,
}

impl Variable {
    pub fn from_identifier(id: Identifier) -> Self {
        Variable { id }
    }
}

impl GenType for Variable {
    fn gen_type(&self, equs: &mut TypeEquations, trs: &TraitsInfo) -> TResult {
        equs.get_type_from_variable(trs, self)
    }
}

impl Transpile for Variable {
    fn transpile(&self, ta: &TypeAnnotation) -> String {
        ta.trans_variable(self)
    }
}

impl MutCheck for Variable {
    fn mut_check(&self, ta: &TypeAnnotation, vars: &mut VariablesInfo) -> Result<MutResult, String> {
        Ok(vars.find_variable(&self.id).unwrap_or(MutResult::NotMut))
    }
}

pub fn parse_variable(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, id) = parse_identifier(s)?;
    Ok((s, UnaryExpr::Variable(Variable { id })))
}

#[derive(Debug)]
pub struct Parentheses {
    pub expr: Expression,
}

impl GenType for Parentheses {
    fn gen_type(&self, equs: &mut TypeEquations, trs: &TraitsInfo) -> TResult {
        self.expr.gen_type(equs, trs)
    }
}

impl Transpile for Parentheses {
    fn transpile(&self, ta: &TypeAnnotation) -> String {
        format!("({})", self.expr.transpile(ta))
    }
}

impl MutCheck for Parentheses {
    fn mut_check(&self, ta: &TypeAnnotation, vars: &mut VariablesInfo) -> Result<MutResult, String> {
        self.expr.mut_check(ta, vars)
    }
}

pub fn parse_parentheses(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, (_, _, expr, _, _)) = tuple((char('('), multispace0, parse_expression, multispace0, char(')')))(s)?;
    Ok((s, UnaryExpr::Parentheses(Parentheses { expr })))
}

pub fn parse_bracket_block(s: &str) -> IResult<&str, UnaryExpr> {
    let(s, (_, _, block, _, _)) = tuple((char('{'), multispace0, parse_block, multispace0, char('}')))(s)?;
    Ok((s, UnaryExpr::Block(block)))
}

pub fn parse_unary_trait_method(ss: &str) -> IResult<&str, UnaryExpr> {
    let (s, (typesign, _)) = tuple((parse_type_sign, multispace0))(ss)?;
    let (s, elems) = many1(tuple((opt(tuple((char('#'), multispace0, parse_trait_spec))), multispace0, tag("::"), multispace0, parse_identifier, multispace0)))(s)?;
    let mut elems = elems.into_iter().map(|(op, _, _, _, id, _)| (op.map(|(_, _, tr_id)| tr_id), id)).collect::<Vec<_>>();
    let (tail_tr_op, tail_id) = elems.pop().unwrap();
    let mut ty = TypeSpec::TypeSign(typesign);
    for (op, ty_id) in elems.into_iter() {
        // TODO: remove unwrap
        ty = TypeSpec::Associated(Box::new(ty), AssociatedType { 
            trait_spec: op.unwrap(),
            type_id: AssociatedTypeIdentifier { id: ty_id },
        });
    }
    Ok((s, UnaryExpr::TraitMethod(ty, tail_tr_op, tail_id)))
}

#[test]
fn parse_unary_expr_test() {
    log::debug!("{:?}", parse_unary_expr("func(1, 2, 3)"));
    log::debug!("{:?}", parse_unary_expr("add(1, add(2, 3), 4)"));
    log::debug!("{:?}", parse_unary_expr("generate_func(91)(1333)"));
    log::debug!("{:?}", parse_unary_expr("MyStruct { a: 1i64 + 2i64, b: val, }"));
    log::debug!("{:?}", parse_unary_expr("generate_func(31 * 91, 210)(1333 / 5 * 3)"));
}
#[test]
fn parse_parentheses_expr_test() {
    log::debug!("{:?}", parse_unary_expr("(1 + 2 + 3)"));
}
#[test]
fn parse_trait_method_test() {
    log::debug!("{:?}", parse_unary_expr("i64#MyTrait.out"));
}
