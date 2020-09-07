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
    for token in &results {
        println!("{:?}", token);
    }
    drop(lexer);

    println!("Lexer errors:\n");
    for error in &errors.borrow().errors {
        println!("{:?}", error);
    }

    println!("Parser output:\n");
    let mut parser = parser::Parser::new(&results, errors.borrow_mut());
    let results = parser.go();
    println!("{:#?}", results);
    drop(parser);

    println!("Parser errors:\n");
    for error in &errors.borrow().errors {
        println!("{:?}", error);
    }

    println!("IR output:\n");
    let unwrapped = results.unwrap();
    let mut ir_generator = ir::IRGenerator::new(&unwrapped, errors.borrow_mut());
    let results = ir_generator.go();
    println!("{:#?}", results);
    // drop(ir_generator);

    println!("IR generation errors:\n");
    let errors = ir_generator.errors;
    for error in &errors.errors {
        println!("{:?}", error);
    }

    println!("Interpreter output:\n");
    let mut interpreter = interpreter::Interpreter::new(&mut ir_generator.env, 0);
    interpreter.go();

    for item in interpreter.stack {
        println!("{:?}", item);
    }

    Ok(())
}
