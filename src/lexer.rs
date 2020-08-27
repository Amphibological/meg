//! This module converts raw Meg source code into a stream of tokens for parsing.

use std::cell::RefMut;

use crate::errors::Errors;

enum TokenKind {
    StringLiteral,
    IntegerLiteral,
    FloatLiteral,

    Identifier,
    Operator,

    Newline,
    EOF,
}

pub struct Token<'t> {
    pub kind: TokenKind,
    pub value: &'t str,
    pub position: usize,
}

enum LexerState {
    Normal,
    String,
    Escape,
    Integer,
    Float,
    Identifier, // or keyword
    Operator,
}

pub struct Lexer<'l> {
    code: &'l str,
    index: usize,
    state: LexerState,
    token: String,
    errors: RefMut<'l, Errors>,
}

impl<'l> Lexer<'l> {
    pub fn new(code: &'l str, errors: RefMut<'l, Errors>) -> Self {
        Lexer {
            code,
            index: 0,
            state: LexerState::Normal,
            token: String::new(),
            errors,
        } 
    }

    pub fn go(&mut self) -> Vec<Token<'l>> {
        todo!();
    }
}
