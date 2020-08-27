//! The main entry point of Meg
//! For now it is mostly for debugging purposes

mod errors;
mod lexer;
mod parser;
mod ir;
mod interpreter;
mod llvm;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::cell::RefCell;

fn main() -> std::io::Result<()> {
    println!("Welcome to Meg!\n");

    let mut file = File::open(env::args().nth(1).unwrap())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let errors = RefCell::new(errors::Errors::new());

    println!("Lexer output:\n");
    let mut lexer = lexer::Lexer::new(&contents, errors.borrow_mut());
    let results = lexer.go();

    println!("Parser output:\n");
    let mut parser = parser::Parser::new(&results, errors.borrow_mut());
    let results = parser.go();

    println!("IR output:\n");
    let mut ir_generator = ir::IRGenerator::new(&results, errors.borrow_mut());
    let results = ir_generator.go();

    // TODO: more here...

    Ok(())
}
