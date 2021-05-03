use std::collections::HashMap;

pub mod associated_type;
pub use associated_type::*;

pub mod candidate;
pub use candidate::*;


use nom::bytes::complete::*;
use nom::character::complete::*;
//use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;

use crate::identifier::{ Identifier, parse_identifier };
//use crate::unary_expr::Variable;
use crate::trans::*;
use crate::func_definition::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TraitId {
    pub id: Identifier,
}

impl Transpile for TraitId {
    fn transpile(&self, _: &mut TypeAnnotation) -> String {
        self.id.into_string()
    }
}

pub fn parse_trait_id(s: &str) -> IResult<&str, TraitId> {
    let (s, id) = parse_identifier(s)?;
    Ok((s, TraitId { id }))
}

#[derive(Debug, Clone)]
pub struct TraitDefinition {
    pub trait_id: TraitId,
    pub asso_ids: Vec<AssociatedTypeIdentifier>,
    pub required_methods: HashMap<Identifier, FuncDefinitionInfo>,
}

#[derive(Debug, Clone)]
pub struct TraitDefinitionInfo {
    pub trait_id: TraitId,
    pub asso_ids: Vec<AssociatedTypeIdentifier>,
    pub required_methods: HashMap<Identifier, FuncDefinitionInfo>,
}

impl TraitDefinition {
    pub fn get_trait_id_pair(&self) -> (TraitId, TraitDefinitionInfo) {
        (self.trait_id.clone(), TraitDefinitionInfo {
            trait_id: self.trait_id.clone(),
            asso_ids: self.asso_ids.clone(),
            required_methods: self.required_methods.clone(),
        })
    }
}

impl Transpile for TraitDefinition {
    fn transpile(&self, ta: &mut TypeAnnotation) -> String {
        format!("template<class Self, class = void> struct {} {{ }};\n", self.trait_id.transpile(ta))
    }
}

pub fn parse_trait_definition(s: &str) -> IResult<&str, TraitDefinition> {
    let (s, (_, _, trait_id, _, _, _, many_types, many_methods, _, _)) = 
        tuple((tag("trait"), space1, parse_trait_id,
            space0, char('{'), space0,
            many0(tuple((tag("type"), space1, parse_associated_type_identifier, space0, char(';'), space0))),
            many0(tuple((parse_func_definition_info, space0, char(';'), space0))),
            space0, char('}')))(s)?;
    let asso_ids = many_types.into_iter().map(|(_, _, id, _, _, _)| id).collect();
    let required_methods = many_methods.into_iter().map(|(info, _, _, _)| (info.func_id.clone(), info)).collect();
    Ok((s, TraitDefinition { trait_id, asso_ids, required_methods }))
}



#[test]
fn parse_trait_definition_test() {
    println!("{:?}", parse_trait_definition("trait MyTrait { type Output; type Input; }"));
}
