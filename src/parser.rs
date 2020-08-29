//! This module converts the token stream from the lexer into an AST.

use std::cell::RefMut;

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
}

#[derive(Debug)]
pub enum DeclarationType {
    Normal,
    Constant,
    Mutable,
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
        typ: Box<Option<NodeContext>>,
        body: Box<Option<NodeContext>>,
        decl_type: DeclarationType,
    },
    FunctionDeclaration {
        name: String,
        arg_types: Vec<NodeContext>,
        arg_names: Vec<String>,
        ret_type: Box<NodeContext>,
        body: Box<NodeContext>,
        decl_type: DeclarationType,
    },
    IfExpression {
        condition: Box<NodeContext>,
        then_body: Box<NodeContext>,
        else_body: Box<NodeContext>,
    },
}

#[derive(Debug)]
pub struct NodeContext {
    node: Node,
    position: usize,
    constant: bool,
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
            if self.tokens[self.index + 1].kind == TokenKind::Colon {
                nodes.push(self.declaration()?);
            } else {
                nodes.push(self.expr(0)?);
            }
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
        let mut decl_type = DeclarationType::Normal;
        if self.try_consume_of_kind(TokenKind::Const).is_some() {
            decl_type = DeclarationType::Constant;
        } else if self.try_consume_of_kind(TokenKind::Mut).is_some() {
            decl_type = DeclarationType::Mutable;
        }

        let name = self.consume_identifier()?;        
        if self.try_consume_of_kind(TokenKind::LParen).is_some() {
            let mut arg_names = vec![];
            let mut arg_types = vec![];

            loop {
                arg_names.push(self.consume_identifier()?);
                self.consume_of_kind(TokenKind::Colon);
                arg_types.push(self.expr(0)?);
                if self.try_consume_of_kind(TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.consume_of_kind(TokenKind::RParen)?;

            self.consume_of_kind(TokenKind::Colon)?;
            let ret_type = self.expr(0)?;

            self.consume_of_kind(TokenKind::Equals)?;
            let body = self.expr(0)?;
            Some(self.in_context(true, Node::FunctionDeclaration {
                name,
                arg_types,
                arg_names,
                ret_type: Box::new(ret_type),
                body: Box::new(body),
                decl_type,
            }))
        } else {
            let typ;
            let body;

            self.consume_of_kind(TokenKind::Colon)?;

            if self.try_consume_of_kind(TokenKind::Equals).is_some() {
                typ = None;
                body = Some(self.expr(0)?);
            } else {
                typ = Some(self.expr(0)?);
                if self.try_consume_of_kind(TokenKind::Equals).is_some() {
                    body = Some(self.expr(0)?);
                } else {
                    body = None;
                }
            }
            Some(self.in_context(true, Node::Declaration {
                name,
                typ: Box::new(typ),
                body: Box::new(body),
                decl_type,
            }))
        }
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
                position,
            } => self.in_context(true, Node::Literal {
                typ: Type::IntLiteral,
                value: int,
            }),
            Token {
                kind: TokenKind::FloatLiteral,
                value: float,
                position,
            } => self.in_context(true, Node::Literal {
                typ: Type::FloatLiteral,
                value: float,
            }),
            Token {
                kind: TokenKind::StringLiteral,
                value: s,
                position,
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
                position,
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
                while self.try_consume_of_kind(TokenKind::Newline).is_some() { }
                block
            },
            Token {
                kind: TokenKind::If,
                ..
            } => {
                self.if_expression()?
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
                TokenKind::EOF
                | TokenKind::Newline
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::Comma
                | TokenKind::Equals
                | TokenKind::LBrace
                | TokenKind::RBrace 
                | TokenKind::Elif
                | TokenKind::Else => break,
                TokenKind::Operator => peeked.value,
                TokenKind::LBracket => "[".to_owned(),
                t => panic!("Bad token: {:?}", t),
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
