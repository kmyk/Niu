use std::collections::HashMap;

use crate::unary_expr::Variable;
use crate::type_id::TypeId;
use crate::func_definition::{ FuncDefinitionInfo, FuncDefinition };
use crate::trans::*;
use crate::traits::*;
use crate::unify::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Type(TypeId),
    Func(Vec<Type>, Box<Type>),
    TypeVariable(TypeVariable),
    AssociatedType(Box<Type>, AssociatedType),
    End,
}

impl Type {
    fn occurs(&self, t: &TypeVariable) -> bool {
        match *self {
            Type::TypeVariable(ref s) if s == t => true,
            Type::Func(ref args, ref ret) => {
                for arg in args.iter() {
                    if arg.occurs(t) { return true; }
                }
                if ret.occurs(t) { return true; }
                false
            }
            Type::AssociatedType(ref ty, _) => {
                ty.as_ref().occurs(t)
            }
            _ => false,
        }
    }

    fn subst(&mut self, theta: &TypeSubst) {
        match *self {
            Type::Func(ref mut args, ref mut ret) => {
                for arg in args.iter_mut() {
                    arg.subst(theta);
                }
                ret.subst(theta);
            }
            Type::Type(_) => {},
            Type::AssociatedType(ref mut ty, _) => {
                ty.as_mut().subst(theta)
            }
            Type::End => {},
            // TypeVariable
            ref mut t => {
                let x = t.clone_type_variable();
                let TypeSubst { tv: y, t: into_t } = theta;
                if x == *y {
                    *t = into_t.clone();
                }
            }
        }
    }

    fn clone_type_variable(&self) -> TypeVariable {
        if let Type::TypeVariable(ref tv) = *self { tv.clone() }
        else { unreachable!("it is not TypeVariable") }
    }
}

impl Transpile for Type {
    fn transpile(&self, ta: &mut TypeAnnotation) -> String {
        match *self {
            Type::Type(ref t) => t.transpile(ta),
            _ => unreachable!("it is not Type"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeVariable {
    Counter(usize),
}

#[derive(Debug)]
pub struct TypeEquation {
    pub left: Type,
    pub right: Type,
}

#[derive(Debug)]
pub struct TypeEquations {
    func: HashMap<Variable, FuncDefinitionInfo>,
    pub cnt: usize,
    variables: Vec<HashMap<Variable, Type>>,
    equs: Vec<TypeEquation>,
}

#[derive(Debug)]
pub struct TypeSubst {
    pub tv: TypeVariable,
    pub t: Type,
}

pub type TResult = Result<Type, String>;

pub trait GenType {
    fn gen_type(&self, equs: &mut TypeEquations) -> TResult;
}

impl TypeEquations {
    pub fn new() -> Self {
        Self {
            func: HashMap::new(),
            equs: Vec::new(),
            cnt: 0,
            variables: Vec::new()
        }
    }
    pub fn add_equation(&mut self, left: Type, right: Type) {
        self.equs.push(TypeEquation { left, right });
    }
    pub fn get_type_variable(&mut self) -> Type {
        let i = self.cnt;
        self.cnt += 1;
        Type::TypeVariable(TypeVariable::Counter(i))
    }
    pub fn into_scope(&mut self) {
        self.variables.push(HashMap::new());
    }
    pub fn out_scope(&mut self) {
        self.variables.pop();
    }
    pub fn regist_variable(&mut self, var: Variable, t: Type) {
        println!("{:?} = {:?}", var, t);
        self.variables.last_mut().unwrap().insert(var.clone(), t.clone());
    }
    pub fn regist_func_info(&mut self, func: &FuncDefinition) {
        let (fvar, finfo) = func.get_func_info();
        self.func.insert(fvar, finfo);
    }
    pub fn get_type_from_variable(&mut self, var: &Variable) -> TResult {
        if let Some(func) = self.func.get(var).cloned() {
            return func.generate_type(self);
        }
        for mp in self.variables.iter().rev() {
            if let Some(t) = mp.get(var) {
                return Ok(t.clone())
            }
        }
        Err(format!("Variable {:?} is not found", var))
    }
    pub fn clear_equations(&mut self) {
        self.equs.clear();
    }
    fn subst(&mut self, theta: &TypeSubst) {
        for TypeEquation { left, right } in self.equs.iter_mut() {
            left.subst(theta);
            right.subst(theta);
        }
    }

    fn solve_associated_type(&mut self, ty: Type, trs: &TraitsInfo) -> Type {
        if let Type::AssociatedType(inner_ty, asso) = ty {
            let inner_ty = self.solve_associated_type(*inner_ty, trs);
            if let Type::Type(_) = inner_ty {
                let AssociatedType { ref trait_id, ref type_id } = asso;
                for ImplTrait { trait_id: _, ref impl_ty, ref asso_defs }
                    in trs.impls.get(trait_id).unwrap().iter() {
                        if let Some(_substs) = impl_ty.gen_type(self).ok().map_or(None, |im_ty| {
                            let mut equs = TypeEquations::new();
                            equs.add_equation(inner_ty.clone(), im_ty);
                            equs.unify(trs).ok()
                        }) {
                            // TODO: use substs for generics
                            // TODO: only first time -> if matched not once, err
                            return asso_defs.get(type_id).unwrap().gen_type(self).unwrap()
                        }
                    }

            }
            Type::AssociatedType(Box::new(inner_ty), asso)
        }
        else {
            ty
        }
    }

    pub fn unify(&mut self, trs: &TraitsInfo) -> Result<Vec<TypeSubst>, String> {
        let mut thetas = Vec::new();
        while let Some(TypeEquation { left, right }) = self.equs.pop() {
            let left = self.solve_associated_type(left, trs);
            let right = self.solve_associated_type(right, trs);
            match (left, right) {
                (l, r) if l == r => {}
                (Type::AssociatedType(b, a), right) => {
                    self.equs.push(TypeEquation { left: Type::AssociatedType(b, a), right });
                }
                (left, Type::AssociatedType(b, a)) => {
                    self.equs.push(TypeEquation { left, right: Type::AssociatedType(b, a) });
                }
                (Type::Func(l_args, l_return), Type::Func(r_args, r_return)) => {
                    if l_args.len() != r_args.len() {
                        Err("length of args is not equal.")?;
                    }
                    for (l, r) in l_args.into_iter().zip(r_args.into_iter()) {
                        self.equs.push(TypeEquation { left: l, right: r });
                    }
                    self.equs.push(
                        TypeEquation { left: *l_return,
                        right: *r_return }
                        );
                }
                (Type::TypeVariable(lv), rt) => {
                    if rt.occurs(&lv) {
                        Err("unification failed, occurs")?;
                    }
                    let th = TypeSubst { tv: lv.clone(), t: rt.clone() };
                    self.subst(&th);
                    for TypeSubst { t, .. } in thetas.iter_mut() {
                        t.subst(&th);
                    }
                    thetas.push(th);
                }
                (rt, Type::TypeVariable(lv)) => {
                    if rt.occurs(&lv) {
                        Err("unification failed, occurs")?;
                    }
                    let th = TypeSubst { tv: lv.clone(), t: rt.clone() };
                    self.subst(&th);
                    for TypeSubst { t, .. } in thetas.iter_mut() {
                        t.subst(&th);
                    }
                    thetas.push(th);
                }
                (l, r) => {
                    Err(format!("unfication failed, {:?} != {:?}", l, r))?
                }
            }
        }
        Ok(thetas)
    }
}

#[test]
fn test_unify() {
    let mut traits_info = TraitsInfo::new();
    traits_info.regist_trait(&parse_trait_definition("trait MyTrait { type Output; }").unwrap().1);
    traits_info.regist_trait_impl(&parse_impl_trait("impl MyTrait for i64 { type Output = bool; }").unwrap().1);
    let mut equs = TypeEquations::new();
    let left = equs.get_type_variable();
    let right = crate::type_spec::parse_type_spec("i64#MyTrait::Output").unwrap().1.gen_type(&mut equs).unwrap();
    equs.add_equation(left, right);
    println!("{:?}", equs.unify(&traits_info));
}

/*
#[test]
fn test_unify() {

    let counter = |i: usize| Type::TypeVariable(TypeVariable::Counter(i));
    let typ = |s: &str| Type::Type(Identifier::from_str(s));
    {
        let mut equs = TypeEquations { equs:
            vec![
                TypeEquation { left: counter(0), right: Type::Type(Identifier::from_str("int")) }
            ],
            cnt: 0, variables: Vec::new()
        };
        println!("{:?}", equs.unify());
    }
    {
        let mut equs = TypeEquations { equs:
            vec![
                TypeEquation { left: counter(0), right: Type::Type(Identifier::from_str("int")) },
                TypeEquation { left: counter(1), right: Type::Type(Identifier::from_str("int")) }
            ],
            cnt: 0, variables: Vec::new()
        };
        println!("{:?}", equs.unify());
    }
    {
        let mut equs = TypeEquations { equs:
            vec![
                TypeEquation { left: counter(0), right: Type::Type(Identifier::from_str("int")) },
                TypeEquation { left: counter(1), right: counter(0) }
            ],
            cnt: 0, variables: Vec::new()
        };
        println!("{:?}", equs.unify());
    }
    {
        let mut equs = TypeEquations { equs:
            vec![
                TypeEquation { left: counter(0), right: Type::Func(vec![typ("int"), typ("bool")], Box::new(typ("i64"))) },
                TypeEquation { left: counter(3), right: counter(4) },
                TypeEquation { left: counter(0), right: Type::Func(vec![counter(1), counter(2)], Box::new(counter(4))) },
            ],
            cnt: 0, variables: Vec::new()
        };
        println!("{:#?}", equs.unify());
    }
    {
        let mut equs = TypeEquations { equs:
            vec![
                TypeEquation { left: counter(0), right: Type::Func(vec![typ("int"), typ("bool")], Box::new(typ("i64"))) },
                TypeEquation { left: counter(3), right: counter(4) },
                TypeEquation { left: counter(0), right: Type::Func(vec![counter(1), counter(2)], Box::new(counter(4))) },
                TypeEquation { left: counter(2), right: typ("int") },
            ],
            cnt: 0, variables: Vec::new()
        };
        println!("{:#?}", equs.unify());
    }
}
*/
