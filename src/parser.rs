//! This module converts the token stream from the lexer into an AST.

use std::cell::RefMut;
use std::fmt;

use crate::{
    errors::Errors,
    lexer::{Token, TokenKind},
};

#[derive(Debug)]
pub enum Type {
    IntLiteral,
    FloatLiteral,
    StrLiteral,
    Undefined,
    Bool,

    Unknown,
}

#[derive(Debug)]
pub enum Node {
    Block {
        nodes: Vec<NodeContext>,
    },
    InfixOp {
        op: String,
        left: Box<NodeContext>,
        right: Box<NodeContext>,
    },
    PrefixOp {
        op: String,
        right: Box<NodeContext>,
    },
    PostfixOp {
        op: String,
        left: Box<NodeContext>,
    },
    IndexOp {
        object: Box<NodeContext>,
        index: Box<NodeContext>,
    },
    Literal {
        typ: Type,
        value: String,
    },
    Call {
        name: String,
        args: Vec<NodeContext>,
    },
    VariableRef {
        name: String,
    },
    Declaration {
        name: String,
        typ: Box<NodeContext>,
        body: Box<NodeContext>,
    },
    IfExpression {
        condition: Box<NodeContext>,
        then_body: Box<NodeContext>,
        else_body: Box<NodeContext>,
    },
    WhileExpression {
        condition: Box<NodeContext>,
        body: Box<NodeContext>,
    },
    Assignment {
        name: String,
        value: Box<NodeContext>,
    },
    FunctionExpression {
        arg_types: Vec<NodeContext>,
        arg_names: Vec<String>,
        ret_types: Vec<NodeContext>,
        body: Box<NodeContext>,
    }
}

pub struct NodeContext {
    pub node: Node,
    pub position: usize,
    pub constant: bool,
}

impl fmt::Debug for NodeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(position {}{}) {:#?}", self.position, if self.constant { ", constant" } else { "" }, self.node) 
    } 
}

pub struct Parser<'p> {
    tokens: &'p [Token],
    index: usize,
    source_position: usize,
    errors: RefMut<'p, Errors>,
}

impl<'p> Parser<'p> {
    pub fn new(tokens: &'p [Token], errors: RefMut<'p, Errors>) -> Self {
        Parser {
            tokens,
            index: 0,
            source_position: 0,
            errors,
        }
    }

    fn consume(&mut self) -> Token {
        self.index += 1;
        self.tokens[self.index - 1].clone()
    }

    fn consume_of_kind(&mut self, kind: TokenKind) -> Option<Token> {
        let peeked = self.peek();
        if peeked.kind == kind {
            Some(self.consume())
        } else {
            self.errors.parser(
                format!("Expected token {:?}, but found {:?} instead", kind, peeked.kind),
                peeked.position,
            );
            None
        }
    }

    fn try_consume_of_kind(&mut self, kind: TokenKind) -> Option<Token> {
        if self.peek().kind == kind {
            Some(self.consume())
        } else {
            None
        }
    }

    fn consume_identifier(&mut self) -> Option<String> {
        let peeked = self.peek();
        if peeked.kind == TokenKind::Identifier {
            Some(self.consume().value)
        } else {
            self.errors.parser(
                format!("Expected an identifier, but found {:?} instead", peeked.kind),
                peeked.position,
            );
            None
        }
    }

    fn try_consume_identifier(&mut self) -> Option<String> {
        if self.peek().kind == TokenKind::Identifier {
            Some(self.consume().value)
        } else {
            None
        }
    }

    fn peek(&self) -> Token {
        self.tokens[self.index].clone()
    }

    // TODO source_position needs to be properly saved and restored

    fn in_context(&mut self, constant: bool, node: Node) -> NodeContext {
        NodeContext {
            node,
            position: self.source_position,
            constant,
        } 
    }

    pub fn go(&mut self) -> Option<NodeContext> {
        let mut nodes = vec![];
        loop {
            nodes.push(
                if self.tokens[self.index + 1].kind == TokenKind::Colon {
                    self.declaration()?
                } else if self.tokens[self.index + 1].kind == TokenKind::Equals {
                    self.assignment()?
                } else {
                    self.expr(0)?
                }
            );
            if self.try_consume_of_kind(TokenKind::EOF).is_some() {
                break;
            }
            self.consume_of_kind(TokenKind::Newline)?;
            if self.try_consume_of_kind(TokenKind::RBrace).is_some() {
                break;
            }
            if self.try_consume_of_kind(TokenKind::EOF).is_some() {
                break;
            }
        }

        Some(self.in_context(false, Node::Block { nodes }))
    }

    fn declaration(&mut self) -> Option<NodeContext> {
        let name = self.consume_identifier()?;        
        self.consume_of_kind(TokenKind::Colon)?;

        let typ;
        let body;

        if self.try_consume_of_kind(TokenKind::Equals).is_some() {
            typ = self.in_context(true, Node::Literal {
                typ: Type::Unknown,
                value: "".to_owned(),
            });
            body = self.expr(0)?;
        } else {
            typ = self.expr(0)?;
            if self.try_consume_of_kind(TokenKind::Equals).is_some() {
                body = self.expr(0)?;
            } else {
                body = self.in_context(true, Node::Literal {
                    typ: Type::Undefined,
                    value: "undef".to_owned(),
                });
            }
        }
        Some(self.in_context(true, Node::Declaration {
            name,
            typ: Box::new(typ),
            body: Box::new(body),
        }))
    }

    fn function_expression(&mut self) -> Option<NodeContext> {
        self.consume_of_kind(TokenKind::LParen);
        let mut arg_names = vec![];
        let mut arg_types = vec![];

        if self.try_consume_of_kind(TokenKind::RParen).is_none() {
            loop {
                arg_names.push(self.consume_identifier()?);
                self.consume_of_kind(TokenKind::Colon)?;
                arg_types.push(self.expr(0)?);
                if self.try_consume_of_kind(TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.consume_of_kind(TokenKind::RParen)?;
        }

        // TODO multiple return types
        let ret_type = self.expr(0)?;

        let body = self.expr(0)?; // TODO this needs to specifically be a block???
        Some(self.in_context(true, Node::FunctionExpression {
            arg_types,
            arg_names,
            ret_types: vec![ret_type],
            body: Box::new(body),
        }))
    }

    fn assignment(&mut self) -> Option<NodeContext> {
        let name = self.consume_identifier()?;
        self.consume_of_kind(TokenKind::Equals)?;
        let value = self.expr(0)?;

        Some(self.in_context(false, Node::Assignment {
            name,
            value: Box::new(value),
        }))
    }

    fn if_expression(&mut self) -> Option<NodeContext> {
        // if doesn't actually consume an if cause it is done for it before calling
        let condition = self.expr(0)?;
        let then_body = self.expr(0)?;
        let else_body;
        if self.try_consume_of_kind(TokenKind::Else).is_some() {
            else_body = self.expr(0)?;
        } else if self.try_consume_of_kind(TokenKind::Elif).is_some() {
            else_body = self.if_expression()?;
        } else {
            else_body = self.in_context(true, Node::Literal { typ: Type::Undefined, value: "undef".to_owned() });
        }

        Some(self.in_context(false, Node::IfExpression {
            condition: Box::new(condition),
            then_body: Box::new(then_body),
            else_body: Box::new(else_body),
        }))
    }

    fn while_expression(&mut self) -> Option<NodeContext> {
        let condition = self.expr(0)?;
        let body = self.expr(0)?;

        Some(self.in_context(false, Node::WhileExpression {
            condition: Box::new(condition),
            body: Box::new(body),
        }))
    }

    fn loop_expression(&mut self) -> Option<NodeContext> {
        let condition = self.in_context(true, Node::Literal { typ: Type::Bool, value: "true".to_owned() });
        let body = self.expr(0)?;

        Some(self.in_context(false, Node::WhileExpression {
            condition: Box::new(condition),
            body: Box::new(body),
        }))
    }

    fn expr(&mut self, min_bp: u8) -> Option<NodeContext> {
        let mut left = match self.consume() {
            Token {
                kind: TokenKind::Identifier,
                value: id,
                ..
            } => {
                if self.peek().kind == TokenKind::LParen {
                    self.consume(); // pass the LParen;
                    let mut args = Vec::new();
                    while self.peek().kind != TokenKind::RParen {
                        args.push(self.expr(0)?);
                        if self.peek().kind != TokenKind::Comma {
                            break;
                        } else {
                            self.consume_of_kind(TokenKind::Comma)?;
                        }
                    }
                    self.consume_of_kind(TokenKind::RParen)?;
                    self.in_context(false, Node::Call {
                        name: id,
                        args,
                    })
                } else {
                    self.in_context(false, Node::VariableRef {
                        name: id,
                    })
                }
            }
            Token {
                kind: TokenKind::IntegerLiteral,
                value: int,
                ..
            } => self.in_context(true, Node::Literal {
                typ: Type::IntLiteral,
                value: int,
            }),
            Token {
                kind: TokenKind::FloatLiteral,
                value: float,
                ..
            } => self.in_context(true, Node::Literal {
                typ: Type::FloatLiteral,
                value: float,
            }),
            Token {
                kind: TokenKind::StringLiteral,
                value: s,
                ..
            } => self.in_context(true, Node::Literal {
                typ: Type::StrLiteral,
                value: s,
            }),
            Token {
                kind: TokenKind::LParen,
                ..
            } => {
                let left = self.expr(0)?;
                self.consume_of_kind(TokenKind::RParen)?;
                left
            }
            Token {
                kind: TokenKind::Operator,
                value: op,
                ..
            } => {
                let ((), right_bp) = prefix_binding_power(&op);
                let right = self.expr(right_bp)?;
                self.in_context(false, Node::PrefixOp {
                    op,
                    right: Box::new(right),
                })
            },
            Token {
                kind: TokenKind::LBrace,
                ..
            } => {
                while self.try_consume_of_kind(TokenKind::Newline).is_some() { }
                let block = self.go()?;
                //while self.try_consume_of_kind(TokenKind::Newline).is_some() { }
                block
            },
            Token {
                kind: TokenKind::If,
                ..
            } => {
                self.if_expression()?
            },
            Token {
                kind: TokenKind::While,
                ..
            } => {
                self.while_expression()?
            },
            Token {
                kind: TokenKind::Loop,
                ..
            } => {
                self.loop_expression()?
            },
            Token {
                kind: TokenKind::Fn,
                ..
            } => {
                self.function_expression()?
            },
            Token {
                kind: TokenKind::EOF,
                position,
                ..
            } => {
                self.errors.parser(
                    "Encountered the end of the file while parsing".to_owned(), position
                );
                return None
            }
            t => panic!("Bad token: {:?}", t),
        };

        loop {
            let peeked = self.peek();
            let op = match peeked.kind {
                TokenKind::Operator => peeked.value,
                TokenKind::LBracket => "[".to_owned(),
                _ => break,
            };

            if let Some((left_bp, ())) = postfix_binding_power(&op) {
                if left_bp < min_bp {
                    break;
                }
                self.consume();

                left = if op == "[" {
                    let right = self.expr(0)?;
                    self.consume_of_kind(TokenKind::RBracket)?;
                    self.in_context(true, Node::IndexOp {
                        object: Box::new(left),
                        index: Box::new(right),
                    })
                } else {
                    self.in_context(true, Node::PostfixOp {
                        op,
                        left: Box::new(left),
                    })
                };
                continue;
            }

            if let Some((left_bp, right_bp)) = infix_binding_power(&op) {
                if left_bp < min_bp {
                    break;
                }
                self.consume();

                let right = self.expr(right_bp)?;
                left = self.in_context(false, Node::InfixOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
                continue;
            }

            break;
        }

        Some(left)
    }
}

fn prefix_binding_power(op: &String) -> ((), u8) {
    match op.as_str() {
        ".." => ((), 1),
        "!" => ((), 8),
        "+" | "-" => ((), 9),
        o => unreachable!(o),
    }
}

fn postfix_binding_power(op: &String) -> Option<(u8, ())> {
    Some(match op.as_str() {
        ".." => (1, ()),
        "[" => (11, ()),
        _ => return None,
    })
}

fn infix_binding_power(op: &String) -> Option<(u8, u8)> {
    Some(match op.as_str() {
        ".." => (1, 2),
        ">" | "<" | ">=" | "<=" | "==" | "!=" => (3, 4),
        "+" | "-" => (5, 6),
        "*" | "/" | "//" => (7, 8),
        _ => return None,
    })
}
