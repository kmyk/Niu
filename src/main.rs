extern crate nom;

pub mod literal;
pub mod expression;
pub mod identifier;
pub mod subseq;

pub mod unary_expr;

pub mod statement;
pub mod substitute;
pub mod let_declaration;

pub mod block;

pub mod type_id;
pub mod type_spec;
pub mod func_definition;

pub mod full_content;

pub mod unify;

pub mod trans;

pub mod traits;

pub mod structs;

pub mod cpp_inline;

pub mod mut_checker;

use crate::trans::Transpile;

fn type_check() -> Result<String, String> {
    let args = std::env::args().collect::<Vec<_>>();
    let filename = args.get(1).ok_or("no filepath")?;
    let mut t = crate::full_content::parse_full_content_from_file(&filename).map_err(|e| format!("{:?}", e))?;
    println!("{:?}", t);
    let ta = t.type_check()?;
    t.mut_check(&ta)?;
    Ok(t.transpile(&ta))
}

fn main() {
    match type_check() {
        Ok(prog) => println!("{}", prog),
        Err(err) => println!("{}", err),
    }
}
