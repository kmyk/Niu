use std::collections::HashMap;

use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;

use crate::identifier::{ Identifier, parse_identifier };
use crate::type_id::{ TypeId, parse_type_id };
use crate::block::{ Block, parse_block };
use crate::unify::*;
use crate::unary_expr::Variable;
use crate::trans::*;
use crate::type_spec::*;
use crate::traits::*;

#[derive(Debug, Clone)]
pub enum SelfArg {
    None,
    SelfType,
}

#[derive(Debug)]
pub struct FuncDefinition {
    pub func_id: Identifier,
    pub generics: Vec<TypeId>,
    pub where_sec: WhereSection,
    pub self_arg: SelfArg,
    pub args: Vec<(Identifier, TypeSpec)>,
    pub return_type: TypeSpec,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct FuncDefinitionInfo {
    pub func_id: Identifier,
    pub generics: Vec<TypeId>,
    pub where_sec: WhereSection,
    pub self_arg: SelfArg,
    pub args: Vec<(Identifier, TypeSpec)>,
    pub return_type: TypeSpec,
}

impl FuncDefinitionInfo {
    pub fn generate_type(&self, before_mp: &GenericsTypeMap, equs: &mut TypeEquations, trs: &TraitsInfo, call_id: &Identifier) -> TResult {
        let mut gen_mp = HashMap::new();
        for (i, g_id) in self.generics.iter().enumerate() {
            let ty_var = call_id.generate_type_variable(i);
            gen_mp.insert(g_id.clone(), ty_var.clone());
        }
        let mp = before_mp.next(gen_mp);
        self.where_sec.regist_equations(&mp, equs, trs)?;
        let args = self.args.iter().map(|(_, t)| t.generics_to_type(&mp, equs, trs)).collect::<Result<Vec<Type>, String>>()?;
        let return_type = self.return_type.generics_to_type(&mp, equs, trs)?;
        Ok(Type::Func(args, Box::new(return_type), None))
    }

    pub fn check_equal(&self, right: &Self, equs: &mut TypeEquations, trs: &TraitsInfo) -> Result<(), String> {
        if self.generics != right.generics {
            Err(format!("generics of method {:?} is not matched", self.func_id))?;
        }
        if !self.where_sec.check_equal(&right.where_sec) {
            Err(format!("where_section of method {:?} is not matched", self.func_id))?;
        }
        let mut trs = trs.into_scope();
        for g_id in self.generics.iter() {
            trs.regist_generics_type(g_id)?;
        }
        let self_args  =  self.args.iter().map(|(_, t)| t.generics_to_type(&GenericsTypeMap::empty(), equs, &trs)).collect::<Result<Vec<Type>, String>>()?;
        let right_args = right.args.iter().map(|(_, t)| t.generics_to_type(&GenericsTypeMap::empty(), equs, &trs)).collect::<Result<Vec<Type>, String>>()?;
        let self_return_type = self.return_type.generics_to_type(&GenericsTypeMap::empty(), equs, &trs)?;
        let right_return_type = right.return_type.generics_to_type(&GenericsTypeMap::empty(), equs, &trs)?;
        equs.add_equation(Type::Func(self_args, Box::new(self_return_type), None), Type::Func(right_args, Box::new(right_return_type), None));
        println!("function {:?} and {:?} are equal unify", self.func_id, right.func_id);
        equs.unify(&mut trs)?;
        Ok(())
    }

    pub fn get_generics_annotation(&self, ta: &TypeAnnotation, call_id: &Identifier) -> String {
        if self.generics.len() > 0 {
            let gen = self.generics.iter().enumerate().map(|(i, _)| { let res = ta.annotation(call_id.get_tag_number(), i); res.transpile(ta) })
                          .collect::<Vec<_>>().join(", ");
            format!("<{}>", gen)
        }
        else {
            "".to_string()
        }
    }
}

impl FuncDefinition {
    pub fn get_func_info(&self) -> (Variable, FuncDefinitionInfo) {
        (Variable { id: self.func_id.clone() },
         FuncDefinitionInfo {
             func_id: self.func_id.clone(),
             generics: self.generics.clone(),
             where_sec: self.where_sec.clone(),
             self_arg: self.self_arg.clone(),
             args: self.args.clone(),
             return_type: self.return_type.clone()
         }
         )
    }
    pub fn unify_definition(&self, equs: &mut TypeEquations, trs: &TraitsInfo) -> Result<Vec<TypeSubst>, String> {
        equs.into_scope();

        let mut trs = trs.into_scope();

        for ty_id in self.generics.iter() {
            trs.regist_generics_type(ty_id)?;
            /* if let Some(trait_id) = trait_id {
                trs.regist_param_candidate(equs, ty_id, trait_id)?;
            }*/
        }

        self.where_sec.regist_candidate(equs, &mut trs)?;

        for (i, t) in self.args.iter() {
            let alpha = i.generate_type_variable(0);
            let t_type = t.generics_to_type(&GenericsTypeMap::empty(), equs, &trs)?; 
            equs.regist_variable(Variable::from_identifier(i.clone()), alpha.clone());
            equs.add_equation(alpha, t_type);
        }
        let result_type = self.block.gen_type(equs, &trs)?;
        let return_t = self.return_type.generics_to_type(&GenericsTypeMap::empty(), equs, &trs)?;
        equs.add_equation(result_type, return_t);

        println!("function {:?} unify", self.func_id);
        let result = equs.unify(&mut trs);

        equs.out_scope();
        result
    }
}


impl Transpile for FuncDefinition {
    fn transpile(&self, ta: &TypeAnnotation) -> String {
        let template_str =
            if self.generics.len() > 0 {
                let gen = self.generics.iter().map(|g| format!("class {}", g.transpile(ta))).collect::<Vec<_>>().join(", ");
                format!("template<{}> ", gen)
            } 
            else {
                "".to_string()
            };

        let return_str = self.return_type.transpile(ta);
        let func_str = self.func_id.into_string();
        let arg_str = self.args.iter().map(|(id, ty)| {
            format!("{} {}", ty.transpile(ta), id.into_string())
        }).collect::<Vec<_>>().join(", ");

        let block_str = self.block.transpile(ta);

        format!("{}{} {}({}) {{\n{}\n}}\n", template_str, return_str, func_str, arg_str, block_str)
    }
}

fn parse_generics_arg(s: &str) -> IResult<&str, (TypeId, Option<TraitId>)> {
    let (s, (id, _, opt)) = tuple((parse_type_id, space0, opt(tuple((char(':'), space0, parse_trait_id)))))(s)?;
    Ok((s, (id, opt.map(|(_, _, tr)| tr))))
}

fn parse_self_arg(s: &str) -> IResult<&str, SelfArg> {
    let (s, op) = opt(tuple((tag("self"), space0, char(','), space0)))(s)?;
    let self_arg = match op {
        None => SelfArg::None,
        Some(_) => SelfArg::SelfType,
    };
    Ok((s, self_arg))
}

pub fn parse_func_definition_info(s: &str) -> IResult<&str, FuncDefinitionInfo> {
    let (s, (_, _, func_id, _, generics_opt, _, _, _, self_arg, op, _, _, _, _, return_type, _, where_sec)) = 
        tuple((tag("fn"), space1, parse_identifier, space0, opt(tuple((char('<'), space0, opt(tuple((parse_type_id, space0, many0(tuple((char(','), space0, parse_type_id, space0))), opt(char(',')), space0))), char('>'), space0))), space0,
            char('('), space0, parse_self_arg,
            opt(tuple((parse_identifier, space0, char(':'), space0, parse_type_spec, space0, many0(tuple((char(','), space0, parse_identifier, space0, char(':'), space0, parse_type_spec, space0))), opt(char(',')), space0))),
            char(')'), space0, tag("->"), space0, parse_type_spec, space0, parse_where_section))(s)?;
    let generics = match generics_opt {
        Some((_, _, generics_opt, _, _)) => {
            match generics_opt {
                Some((arg0, _, many, _, _)) => {
                    let mut vec = vec![arg0];
                    for (_, _, arg, _) in many {
                        vec.push(arg);
                    }
                    vec
                }
                None => Vec::new(),
            }
        }
        None => Vec::new(),
    };
    let args = match op {
        Some((arg0, _, _, _, ty0, _, many, _, _)) => {
            let mut args = vec![(arg0, ty0)];
            for (_, _, arg, _, _, _, ty, _) in many {
                args.push((arg, ty));
            }
            args
        }
        None => Vec::new(),
    };
    Ok((s, FuncDefinitionInfo { func_id, generics, where_sec, self_arg, args, return_type }))
}

pub fn parse_func_definition(s: &str) -> IResult<&str, FuncDefinition> {
    let (s, (info, _, _, block, _)) = tuple((parse_func_definition_info, space0, char('{'), parse_block, char('}')))(s)?;
    Ok((s, FuncDefinition {
        func_id: info.func_id,
        generics: info.generics,
        where_sec: info.where_sec,
        self_arg: info.self_arg,
        args: info.args,
        return_type: info.return_type,
        block
    }))
}


#[test]
fn parse_func_definition_test() {
    println!("{:?}", parse_func_definition("fn func(x: i64) -> i64 { let y = x * x; y + x }"));
    println!("{:?}", parse_func_definition("fn func2<t>(x: t) -> t { x }"));
    println!("{:?}", parse_func_definition("fn func3<x, y, z>(x: x) -> z { x }"));
}
#[test]
fn parse_func_definition2_test() {
    println!("{:?}", parse_func_definition("fn func2<t>(x: t) -> t where t: MyTraits{ x }"));
    println!("{:?}", parse_func_definition_info("fn nest_out<T>(t: T) -> T#MyTrait::Output#MyTrait::Output where T: MyTrait, T#MyTrait::Output: MyTrait"));
}

#[test]
fn parse_func_definition_self_test() {
    println!("{:?}", parse_func_definition("fn func2<t>(self, x: t) -> t where t: MyTraits{ x }"));
}
