//! This module interpretes Meg IR into constant expressions for CTFE

//use crate::ir::{Environment, Value};
//
//pub struct Interpreter<'i> {
//    env: &'i mut Environment,
//    stack: Vec<Value>,
//
//    current_function: usize,
//    current_block: usize,
//    current_instruction: usize,
//}
//
//impl<'i> Interpreter<'i> {
//    pub fn new(env: &'i mut Environment, func_id: usize) -> Self {
//        Interpreter {
//            env,
//            stack: vec![],
//
//            current_function: func_id,
//            current_block:  f
//        } 
//    }
//}
