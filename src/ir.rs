//! This module converts the AST of a Meg program into Meg IR.

use std::cell::RefMut;

use crate::{
    errors::Errors,
    parser::Node,
};

pub struct IRProgram {
    // ???
}

pub struct IRGenerator<'i> {
    ast: &'i Node,
    errors: RefMut<'i, Errors>,
}

impl<'i> IRGenerator<'i> {
    pub fn new(ast: &'i Node, errors: RefMut<'i, Errors>) -> Self {
        IRGenerator {
            ast,
            errors,
        }
    }

    pub fn go(&mut self) -> IRProgram {
        todo!();
    }
}
