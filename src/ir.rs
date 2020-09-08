//! This module converts the AST of a Meg program into Meg IR.

use std::cell::RefMut;
use std::collections::HashMap;
use std::fmt;

use crate::{
    errors::Errors,
    parser::{
        Node,
        NodeContext,
        Type,
    },
};

#[derive(Debug, Clone)]
pub enum CompareType {
    EQ,
    NE,
    LT,
    GT,
    LE,
    GE,
}

#[derive(Debug, Clone)]
pub enum InstructionKind {
    ConstBool(bool),
    ConstInt(i128),
    ConstFloat(f64),
    ConstString(String),

    Allocate(String),
    Push(String),
    Pop(String),

    Add,
    Subtract,
    Multiply,
    ExactDivide,
    FloorDivide,
    Negate,
    Test(CompareType),

    Call,
    Return,
    BranchIf(usize, usize),
    Jump(usize),

    GetFunction(usize),
}

#[derive(Clone)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub constant: bool,
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}{}", self.kind, if self.constant { " constant" } else { "" })
    } 
}

#[derive(Clone)]
pub struct BasicBlock {
    pub id: usize,
    pub instructions: Vec<Instruction>,
}

impl fmt::Debug for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.id)?;
        for ins in &self.instructions {
            writeln!(f, "{:?}", ins)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Function {
    pub id: usize,
    pub args: usize,
    pub retvals: usize,
    pub blocks: Vec<BasicBlock>,
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "function {} (args: {}, retvals: {})", self.id, self.args, self.retvals)?;
        for block in &self.blocks {
            writeln!(f, "{:?}", block)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(String),

    Function(Function),
}

pub type Scope = HashMap<String, Value>;

#[derive(Debug)]
pub struct Environment {
    pub scopes: Vec<Scope>,
    pub functions: HashMap<usize, Function>, // TODO this could probably be optimized down to a Vec??
}

impl Environment {
    pub fn current_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }
}

pub struct IRGenerator<'i> {
    ast: &'i NodeContext,
    pub errors: RefMut<'i, Errors>,
    pub env: Environment,
    next_func_id: usize,
    next_block_id: usize,
}

impl<'i> IRGenerator<'i> {
    pub fn new(ast: &'i NodeContext, errors: RefMut<'i, Errors>) -> Self {
        IRGenerator {
            ast,
            errors,
            env: Environment {
                scopes: vec![new_global_scope()],
                functions: HashMap::new(),
            },
            next_func_id: 0,
            next_block_id: 0,
        }
    }

    pub fn go(&mut self) -> &Environment {
        let mut top_level_fn = Function {
            id: self.get_next_func_id(),
            args: 0,
            retvals: 0,
            blocks: vec![
                BasicBlock {
                    id: self.get_next_block_id(),
                    instructions: vec![],
                },
                BasicBlock {
                    id: self.get_next_block_id(),
                    instructions: vec![],
                },
            ],
        };

        self.node(&mut top_level_fn, self.ast);

        top_level_fn.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Push("main".to_owned()),
                constant: false,
            }
        );

        top_level_fn.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Call,
                constant: false,
            }
        );

        self.env.functions.insert(0, top_level_fn);

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
            } => self.declaration(func, name, typ, body, node.constant),
            FunctionExpression {
                arg_types,
                arg_names,
                ret_types,
                body,
            } => self.function_expression(func, arg_types, arg_names, ret_types, body, node.constant),
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

    fn infix_op(&mut self, func: &mut Function, op: &str, left: &Box<NodeContext>, right: &Box<NodeContext>, constant: bool) {
        self.node(func, left);
        self.node(func, right);

        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: match op {
                    "+" => InstructionKind::Add,
                    "-" => InstructionKind::Subtract,
                    "*" => InstructionKind::Multiply,
                    "/" => InstructionKind::ExactDivide,
                    "//" => InstructionKind::FloorDivide,

                    "==" => InstructionKind::Test(CompareType::EQ),
                    "!=" => InstructionKind::Test(CompareType::NE),
                    "<" => InstructionKind::Test(CompareType::LT),
                    ">" => InstructionKind::Test(CompareType::GT),
                    "<=" => InstructionKind::Test(CompareType::LE),
                    ">=" => InstructionKind::Test(CompareType::GE),

                    _ => unreachable!(),
                },
                constant,
            }
        );
    }

    fn prefix_op(&mut self, func: &mut Function, op: &str, right: &Box<NodeContext>, constant: bool) {
        self.node(func, right);

        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: match op {
                    "-" => InstructionKind::Negate,
                    _ => unreachable!(),
                },
                constant,
            }
        );
 
    }

    fn postfix_op(&mut self, func: &mut Function, op: &str, left: &Box<NodeContext>, constant: bool) {
        todo!("{:?}{:?}{:?}{:?}", func, op, left, constant)
    }

    fn index_op(&mut self, func: &mut Function, object: &Box<NodeContext>, index: &Box<NodeContext>, constant: bool) {
        todo!("{:?}{:?}{:?}{:?}", func, object, index, constant)
    }

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

    fn declaration(&mut self,
        func: &mut Function,
        name: &str,
        typ: &Box<NodeContext>,
        body: &Box<NodeContext>,
        constant: bool
    ) {
        self.node(func, typ);
        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Allocate(name.into()),
                constant,
            }
        );
        self.node(func, body);
        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Pop(name.into()),
                constant,
            }
        );
    }

    fn function_expression(&mut self,
        func: &mut Function,
        arg_types: &[NodeContext],
        _arg_names: &[String],
        ret_types: &[NodeContext],
        body: &Box<NodeContext>,
        constant: bool
    ) {
        let mut new_func = Function {
            id: self.get_next_func_id(),
            args: arg_types.len(),
            retvals: ret_types.len(),
            blocks: vec![
                BasicBlock {
                    id: self.get_next_block_id(),
                    instructions: vec![],
                },
                BasicBlock {
                    id: self.get_next_block_id(),
                    instructions: vec![],
                },
            ],
        };

        self.node(&mut new_func, body);

        new_func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Return,
                constant,
            }
        );

        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::GetFunction(new_func.id),
                constant,
            }
        );

        self.env.functions.insert(new_func.id, new_func);
    }

    fn if_expression(
        &mut self,
        func: &mut Function,
        condition: &Box<NodeContext>,
        then_body: &Box<NodeContext>,
        else_body: &Box<NodeContext>,
        constant: bool
    ) {
        self.node(func, condition);
        let then_block_id = self.get_next_block_id();
        let else_block_id = self.get_next_block_id();
        let end_block_id = self.get_next_block_id();

        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::BranchIf(then_block_id, else_block_id),
                constant,
            }
        );

        func.blocks.push(BasicBlock {
            id: then_block_id,
            instructions: vec![],
        }); 
        
        self.node(func, then_body);
        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Jump(end_block_id),
                constant,
            }
        );

        func.blocks.push(BasicBlock {
            id: else_block_id,
            instructions: vec![],
        }); 

        self.node(func, else_body);

        func.blocks.last_mut().unwrap().instructions.push(
            Instruction {
                kind: InstructionKind::Jump(end_block_id),
                constant,
            }
        );

        func.blocks.push(BasicBlock {
            id: end_block_id,
            instructions: vec![],
        }); 
    }

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

    fn get_next_block_id(&mut self) -> usize {
        self.next_block_id += 1;
        self.next_block_id - 1
    }

    fn get_next_func_id(&mut self) -> usize {
        self.next_func_id += 1;
        self.next_func_id - 1
    }
}

fn new_global_scope() -> Scope {
    let mut scope = HashMap::new();
    scope.insert("true".to_owned(), Value::Bool(true));
    scope.insert("false".to_owned(), Value::Bool(false));
    scope.insert("i32".to_owned(), Value::Bool(false));
    scope
}

