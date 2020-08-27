//! This module converts raw Meg source code into a stream of tokens for parsing.

use std::cell::RefMut;
use std::iter::FromIterator;

use crate::errors::Errors;

#[derive(Debug)]
pub enum TokenKind {
    StringLiteral,
    IntegerLiteral,
    FloatLiteral,

    Identifier,
    Operator,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Colon,
    Equals,
    Comma,

    Newline,
    EOF,
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub position: usize,
}

#[derive(PartialEq)]
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
    code: Vec<char>,
    index: usize,
    state: LexerState,
    errors: RefMut<'l, Errors>,
}

impl<'l> Lexer<'l> {
    pub fn new(code: &'l str, errors: RefMut<'l, Errors>) -> Self {
        Lexer {
            code: code.chars().collect(),
            index: 0,
            state: LexerState::Normal,
            errors,
        } 
    }

    pub fn go(&mut self) -> Vec<Token> {
        let mut token = vec![];
        let mut tokens = vec![];

        loop {
            let ch = *self.code.get(self.index).unwrap_or(&'\0');

            match self.state {
                LexerState::Normal => {
                    if ch == '\n' {
                        tokens.push(Token {
                            kind: TokenKind::Newline,
                            value: "\n".to_owned(),
                            position: self.index,
                        });
                    } else if ch.is_whitespace() || ch == '\0' {
                    
                    } else if ch == '"' {
                        self.state = LexerState::String;
                    } else if ch.is_digit(10) {
                        token.push(ch);   
                        self.state = LexerState::Integer;
                    } else if is_special(ch) {
                        tokens.push(Token {
                            kind: match ch {
                                '(' => TokenKind::LParen,
                                ')' => TokenKind::RParen,
                                '[' => TokenKind::LBracket,
                                ']' => TokenKind::RBracket,
                                '{' => TokenKind::LBrace,
                                '}' => TokenKind::RBrace,
                                ':' => TokenKind::Colon,
                                '=' => TokenKind::Equals,
                                ',' => TokenKind::Comma,
                                _ => unreachable!(),
                            },
                            value: ch.to_string(),
                            position: self.index,
                        });
                    } else if ch.is_ascii_punctuation() {
                        token.push(ch);
                        self.state = LexerState::Operator;
                    } else if ch.is_alphabetic() || ch == '_' {
                        token.push(ch);
                        self.state = LexerState::Identifier;
                    } else {
                        self.errors.lexer(
                            format!("Found invalid character {} ({})", ch, ch),
                            self.index,
                        );
                    }
                }
                LexerState::Integer => {
                    if ch.is_digit(10) {
                        token.push(ch);
                    } else if ch == '.' {
                        token.push(ch);
                        self.state = LexerState::Float;
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::IntegerLiteral,
                            value: String::from_iter(token.clone()),
                            position: 0,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                        continue;
                    }
                }
                LexerState::Float => {
                    if ch.is_digit(10) { token.push(ch); } else if ch == '.' {
                        self.state = LexerState::Normal;
                        continue;
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::FloatLiteral,
                            value: String::from_iter(token.clone()),
                            position: 0,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                        continue;
                    }

                }
                LexerState::String => {
                    if ch == '"' {
                        tokens.push(Token {
                            kind: TokenKind::StringLiteral,
                            value: String::from_iter(token.clone()),
                            position: 0,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                    } else if ch == '\\' {
                        token.push(ch);
                        self.state = LexerState::Escape;
                    } else {
                        token.push(ch);
                    }
                }
                LexerState::Operator => {
                    if ch.is_ascii_punctuation() {
                        token.push(ch);
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Operator,
                            value: String::from_iter(token.clone()),
                            position: 0,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                        continue;
                    }
                }
                LexerState::Identifier => {
                    if ch.is_alphanumeric() || ch == '_' {
                        token.push(ch);
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::Identifier,
                            value: String::from_iter(token.clone()),
                            position: 0,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                        continue;
                    }
                }
                LexerState::Escape => {
                    todo!();
                }
            }

            if self.index >= self.code.len() {
                if self.state == LexerState::String {
                    self.errors.lexer(
                        format!("Found EOF while parsing a string literal \"{}\"", String::from_iter(token.clone())),
                        self.index,
                    );
                }

                tokens.push(Token {
                    kind: TokenKind::EOF,
                    value: "".to_owned(),
                    position: self.index,
                });
                break;
            }

            self.index += 1;
        }

        tokens
    }
}

fn is_special(ch: char) -> bool {
    ['(', ')', '[', ']', '{', '}', ':', '=', ','].contains(&ch)
}
