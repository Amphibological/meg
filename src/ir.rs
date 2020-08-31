//! This module converts the AST of a Meg program into Meg IR.

use std::cell::RefMut;
use std::collections::HashMap;

use crate::{
    errors::Errors,
    parser::{
        Node,
        NodeContext,
        Type,
        DeclarationType,
    },
};

#[derive(Debug)]
pub enum InstructionKind {
    ConstBool(bool),
    ConstInt(i128),
    ConstFloat(f64),
    ConstString(String),

    Push(String),
    Pop(String),

    Call,
}

#[derive(Debug)]
pub struct Instruction {
    kind: InstructionKind,
    constant: bool,
}

#[derive(Debug)]
pub struct BasicBlock {
    id: usize,
    instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub struct Function {
    name: String,
    args: usize,
    retvals: usize,
    blocks: Vec<BasicBlock>,
}

#[derive(Debug)]
pub enum Value {
    Function(Function),
}

pub type Scope = HashMap<String, Value>;

#[derive(Debug)]
pub struct Environment {
    scopes: Vec<Scope>,
}

pub struct IRGenerator<'i> {
    ast: &'i NodeContext,
    errors: RefMut<'i, Errors>,
    env: Environment,
}

impl<'i> IRGenerator<'i> {
    pub fn new(ast: &'i NodeContext, errors: RefMut<'i, Errors>) -> Self {
        IRGenerator {
            ast,
            errors,
            env: Environment {
                scopes: vec![new_global_scope()],
            },
        }
    }

    pub fn go(&mut self) -> &Environment {
        self.node(&mut Function {
            name: "dummy".to_owned(), 
            args: 0,
            retvals: 0,
            blocks: vec![],
        }, self.ast);

        &self.env
    }

    fn node(&mut self, func: &mut Function, node: &NodeContext) {
        use Node::*;
        match &node.node {
            Block {
                nodes,
            } => self.block(func, nodes, node.constant),
            InfixOp {
                op,
                left,
                right,
            } => self.infix_op(func, op, left, right, node.constant),
            PrefixOp {
                op,
                right,
            } => self.prefix_op(func, op, right, node.constant),
            PostfixOp {
                op,
                left,
            } => self.postfix_op(func, op, left, node.constant),
            IndexOp {
                object,
                index,
            } => self.index_op(func, object, index, node.constant),
            Literal {
                typ,
                value,
            } => self.literal(func, typ, value, node.constant),
            Call {
                name,
                args,
            } => self.call(func, name, args, node.constant),
            VariableRef {
                name,
            } => self.variable_ref(func, name, node.constant),
            Declaration {
                name,
                typ,
                body,
                decl_type,
            } => self.declaration(func, name, typ, body, decl_type, node.constant),
            FunctionDeclaration {
                name,
                arg_types,
                arg_names,
                ret_type,
                body,
            } => self.function_declaration(func, name, arg_types, arg_names, ret_type, body, node.constant),
            IfExpression {
                condition,
                then_body,
                else_body,
            } => self.if_expression(func, condition, then_body, else_body, node.constant),
            WhileExpression {
                condition,
                body,
            } => self.while_expression(func, condition, body, node.constant),
            Assignment {
                name,
                value,
            } => self.assignment(func, name, value, node.constant),
        }
    }

    fn block(&mut self, func: &mut Function, nodes: &[NodeContext], _constant: bool) {
        for node in nodes {
            self.node(func, node);
        }
    }

    fn infix_op(&mut self, func: &mut Function, op: &str, left: &Box<NodeContext>, right: &Box<NodeContext>, constant: bool) {}
    fn prefix_op(&mut self, func: &mut Function, op: &str, right: &Box<NodeContext>, constant: bool) {}
    fn postfix_op(&mut self, func: &mut Function, op: &str, left: &Box<NodeContext>, constant: bool) {}
    fn index_op(&mut self, func: &mut Function, object: &Box<NodeContext>, index: &Box<NodeContext>, constant: bool) {}

    fn literal(&mut self, func: &mut Function, typ: &Type, value: &str, constant: bool) {
        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: match typ {
                    Type::Bool => InstructionKind::ConstBool(match value {
                        "true" => true,
                        "false" => false,
                        _ => unreachable!(),
                    }),
                    Type::IntLiteral => InstructionKind::ConstInt(value.parse().unwrap()),
                    Type::FloatLiteral => InstructionKind::ConstFloat(value.parse().unwrap()),
                    Type::StrLiteral => InstructionKind::ConstString(value.to_owned()),
                    _ => todo!(),
                },
                constant,
            }
        );
    }

    fn call(&mut self, func: &mut Function, name: &str, args: &[NodeContext], constant: bool) {
        for arg in args {
            self.node(func, arg);
        }

        func.blocks.last_mut().unwrap().instructions.append(&mut vec![
            Instruction {
                kind: InstructionKind::Push(name.into()),
                constant,
            },
            Instruction {
                kind: InstructionKind::Call,
                constant,
            },
        ]);
    }

    fn variable_ref(&mut self, func: &mut Function, name: &str, constant: bool) {
        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Push(name.into()),
                constant,
            }
        );
    }

    fn declaration(&mut self, func: &mut Function, name: &str, typ: &Box<NodeContext>, body: &Box<NodeContext>, decl_type: &DeclarationType, constant: bool) {}

    fn function_declaration(&mut self,
        _func: &mut Function,
        name: &str,
        arg_types: &[NodeContext],
        _arg_names: &[String],
        _ret_type: &Box<NodeContext>,
        body: &Box<NodeContext>,
        _constant: bool
    ) {
        let mut new_func = Function {
            name: name.to_owned(),
            args: arg_types.len(),
            retvals: 1,
            blocks: vec![
                BasicBlock {
                    id: 0,
                    instructions: vec![],
                },
                BasicBlock {
                    id: 1,
                    instructions: vec![],
                },
            ],
        };


        self.env.scopes.push(new_local_scope());
        self.node(&mut new_func, body);
        let scope_index = self.env.scopes.len() - 2;
        self.env.scopes[scope_index].insert(new_func.name.clone(), Value::Function(new_func));
    }

    fn if_expression(&mut self, func: &mut Function, condition: &Box<NodeContext>, then_body: &Box<NodeContext>, else_body: &Box<NodeContext>, constant: bool) {}
    fn while_expression(&mut self, func: &mut Function, condition: &Box<NodeContext>, body: &Box<NodeContext>, constant: bool) {}

    fn assignment(&mut self, func: &mut Function, name: &str, value: &Box<NodeContext>, constant: bool) {
        self.node(func, value);

        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Pop(name.into()),
                constant,
            }
        );

    }

    fn current_scope(&mut self) -> &mut Scope {
        self.env.scopes.last_mut().unwrap()
    }
}

fn new_global_scope() -> Scope {
    HashMap::new()
}

fn new_local_scope() -> Scope {
    HashMap::new()
}
