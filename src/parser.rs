//! This module converts the token stream from the lexer into an AST.

use std::cell::RefMut;

use crate::{
    errors::Errors,
    lexer::Token,
};

pub enum Node {
    Block {
        nodes: Vec<Node>,
        position: usize,
        compile_known: bool,
    }
}

pub struct Parser<'p> {
    tokens: &'p [Token],
    index: usize,
    errors: RefMut<'p, Errors>,
}

impl<'p> Parser<'p> {
    pub fn new(tokens: &'p [Token], errors: RefMut<'p, Errors>) -> Self {
        Parser {
            tokens,
            index: 0,
            errors,
        }
    }

    pub fn go(&mut self) -> Node {
        todo!();
    }
}
