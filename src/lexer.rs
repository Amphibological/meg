//! This module converts raw Meg source code into a stream of tokens for parsing.

use std::cell::RefMut;
use std::iter::FromIterator;

use crate::errors::Errors;

#[derive(Debug, PartialEq, Clone)]
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

    Fn,
    If,
    Elif,
    Else,
    While,
    Loop,

    Newline,
    EOF,
}

#[derive(Debug, PartialEq, Clone)]
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
        let mut start_position = 0usize;
        let mut tokens: Vec<Token> = vec![];

        loop {
            let ch = *self.code.get(self.index).unwrap_or(&'\0');
            match self.state {
                LexerState::Normal => {
                    if ch == '\n' && tokens.last()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Newline) != TokenKind::Newline {

                        tokens.push(Token {
                            kind: TokenKind::Newline,
                            value: "\n".to_owned(),
                            position: self.index,
                        });
                    } else if ch.is_whitespace() || ch == '\0' {
                    
                    } else if ch == '"' {
                        self.state = LexerState::String;
                        start_position = self.index;
                    } else if ch.is_digit(10) {
                        token.push(ch);   
                        self.state = LexerState::Integer;
                        start_position = self.index;
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
                        start_position = self.index;
                    } else if ch.is_alphabetic() || ch == '_' {
                        token.push(ch);
                        self.state = LexerState::Identifier;
                        start_position = self.index;
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
                            position: start_position,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                        continue;
                    }
                }
                LexerState::Float => {
                    if ch.is_digit(10) {
                        token.push(ch);
                    } else if ch == '.' {
                        self.state = LexerState::Normal;
                        continue;
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::FloatLiteral,
                            value: String::from_iter(token.clone()),
                            position: start_position,
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
                            position: start_position,
                        });
                        token.clear();
                        self.state = LexerState::Normal;
                    } else if ch == '\\' {
                        token.push(ch);
                        self.state = LexerState::Escape;
                    } else if ch == '\0' {

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
                            position: start_position,
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
                        tokens.push(try_convert_keyword(String::from_iter(token.clone()), start_position).unwrap_or(Token {
                            kind: TokenKind::Identifier,
                            value: String::from_iter(token.clone()),
                            position: start_position,
                        }));
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

fn try_convert_keyword(s: String, position: usize) -> Option<Token> {
    Some(Token {
        kind: match s.as_str() {
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            _ => return None,
        },
        value: s,
        position,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn lexer_results(contents: &'static str) -> Vec<Token> {
        let errors = RefCell::new(crate::errors::Errors::new());
        let mut lexer = Lexer::new(&contents, errors.borrow_mut());
        lexer.go()
    }

    fn lexer_errors(contents: &'static str) -> Vec<crate::errors::Error> {
        let errors = RefCell::new(crate::errors::Errors::new());
        {
            let mut lexer = Lexer::new(&contents, errors.borrow_mut());
            let _ = lexer.go();
        }
        let borrowed = errors.borrow();
        borrowed.errors.clone()
    }

    #[test]
    fn string_literal() {
        assert_eq!(lexer_results(r#""hello world" more_stuff"#), vec![
            Token {
                kind: TokenKind::StringLiteral,
                value: "hello world".to_owned(),
                position: 0,
            },
            Token {
                kind: TokenKind::Identifier,
                value: "more_stuff".to_owned(),
                position: 14,
            },
            Token {
                kind: TokenKind::EOF,
                value: "".to_owned(),
                position: 24,
            },
        ]);
    }

    #[test]
    fn string_literal_ends_too_early() {
        assert_eq!(lexer_results(r#""hello world more_stuff"#), vec![
            Token {
                kind: TokenKind::EOF,
                value: "".to_owned(),
                position: 23,
            }
        ]);
        assert_eq!(lexer_errors(r#""hello world more_stuff"#), vec![
            crate::errors::Error::Lexer {
                message: "Found EOF while parsing a string literal \"hello world more_stuff\"".to_owned(),
                position: 23,
            }
        ]);
    }
}
