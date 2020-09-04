//! This module interpretes Meg IR into constant expressions for CTFE

use crate::ir::{Environment, Value, InstructionKind, CompareType};

pub struct Interpreter<'i> {
    env: &'i mut Environment,
    pub stack: Vec<Value>,

    current_function: usize,
    current_block: usize,
    current_instruction: usize,
    finished: bool,
}

impl<'i> Interpreter<'i> {
    pub fn new(env: &'i mut Environment, func_id: usize) -> Self {
        let first_block = env.functions[&func_id].blocks[0].id;

        Interpreter {
            env,
            stack: vec![],

            current_function: func_id,
            current_block: first_block,
            current_instruction: 0,
            finished: false,
        } 
    }

    fn advance(&mut self) {
        self.current_instruction += 1;
        if self.current_instruction >= self.env.functions[&self.current_function].blocks[self.current_block].instructions.len() {
            self.current_block += 1;
        }

        if self.current_block >= self.env.functions[&self.current_function].blocks.len() {
            self.finished = true;
        }
    }

    pub fn go(&mut self) { // TODO at some point this will return something???
        loop {
            use InstructionKind::*;
            let ins = if self.finished {
                return;
            } else {
                &self.env.functions[&self.current_function]
                    .blocks[self.current_block]
                    .instructions[self.current_instruction]
            };
            
            match &ins.kind.clone() {
                ConstBool(value) => self.const_bool(value),
                ConstInt(value) => self.const_int(value),
                ConstFloat(value) => self.const_float(value),
                ConstString(value) => self.const_string(value),

                Allocate(name) => self.allocate(name),
                Push(name) => self.push(name),
                Pop(name) => self.pop(name),

                Add => self.add(),
                Subtract => self.subtract(),
                Multiply => self.multiply(),
                ExactDivide => self.exact_divide(),
                FloorDivide => self.floor_divide(),
                Negate => self.negate(),
                Test(compare_type) => self.test(compare_type),

                Call => self.call(),
                BranchIf(then_block, else_block) => self.branch_if(then_block, else_block),
                Jump(block) => self.jump(block),

                GetFunction(func) => self.get_function(func),
            }
        }
    }

    fn const_bool(&mut self, value: &bool) {
        self.stack.push(Value::Bool(*value));
        self.advance();
    }

    fn const_int(&mut self, value: &i128) {
        self.stack.push(Value::Integer(*value));
        self.advance();
    }

    fn const_float(&mut self, value: &f64) {
        self.stack.push(Value::Float(*value));
        self.advance();
    }

    fn const_string(&mut self, value: &str) {
        self.stack.push(Value::String(value.to_owned()));
        self.advance();
    }

    fn allocate(&mut self, name: &str) {
        self.env.current_scope().insert(name.to_owned(), self.stack.pop().unwrap());
        self.advance();
    }

    fn push(&mut self, name: &str) {
        dbg!(name);
        self.stack.push(self.env.current_scope()[name].clone());
        self.advance();
    }

    fn pop(&mut self, name: &str) {
        *self.env.current_scope().get_mut(name).unwrap() = self.stack.pop().unwrap();
        self.advance();
    }

    fn add(&mut self) {
        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        self.stack.push(match v1 {
            Value::Integer(i1) => if let Value::Integer(i2) = v2 {
                Value::Integer(i1 + i2)
            } else {
                panic!()
            },
            Value::Float(f1) => if let Value::Float(f2) = v2 {
                Value::Float(f1 + f2)
            } else {
                panic!()
            },
            _ => panic!(),
        });
        self.advance();
    }

    fn subtract(&mut self) {
        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        self.stack.push(match v1 {
            Value::Integer(i1) => if let Value::Integer(i2) = v2 {
                Value::Integer(i2 - i1)
            } else {
                panic!()
            },
            Value::Float(f1) => if let Value::Float(f2) = v2 {
                Value::Float(f2 - f1)
            } else {
                panic!()
            },
            _ => panic!(),
        });
        self.advance();
    }

    fn multiply(&mut self) {
        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        self.stack.push(match v1 {
            Value::Integer(i1) => if let Value::Integer(i2) = v2 {
                Value::Integer(i1 * i2)
            } else {
                panic!()
            },
            Value::Float(f1) => if let Value::Float(f2) = v2 {
                Value::Float(f1 * f2)
            } else {
                panic!()
            },
            _ => panic!(),
        });
        self.advance();
    }

    fn exact_divide(&mut self) {
        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        self.stack.push(match v1 {
            Value::Integer(i1) => if let Value::Integer(i2) = v2 {
                Value::Float(i2 as f64 / i1 as f64)
            } else {
                panic!()
            },
            Value::Float(f1) => if let Value::Float(f2) = v2 {
                Value::Float(f2 / f1)
            } else {
                panic!()
            },
            _ => panic!(),
        });
        self.advance();
    }

    fn floor_divide(&mut self) {
        let v1 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        self.stack.push(match v1 {
            Value::Integer(i1) => if let Value::Integer(i2) = v2 {
                Value::Integer(i2 / i1)
            } else {
                panic!()
            },
            Value::Float(f1) => if let Value::Float(f2) = v2 {
                Value::Integer((f2 / f1).floor() as i128)
            } else {
                panic!()
            },
            _ => panic!(),
        });
        self.advance();
    }

    fn negate(&mut self) {
        let v1 = self.stack.pop().unwrap();
        self.stack.push(match v1 {
            Value::Integer(i1) => Value::Integer(-i1),
            Value::Float(f1) => Value::Float(-f1),
            _ => panic!(),
        });
        self.advance();
    }

    fn test(&mut self, compare_type: &CompareType) {
        let v1 = self.stack.pop().unwrap();  
        let v2 = self.stack.pop().unwrap();  

        self.stack.push(match v1 {
            Value::Integer(i1) => if let Value::Integer(i2) = v2 {
                Value::Bool(match compare_type {
                    CompareType::EQ => i1 == i2,
                    _ => unreachable!(),
                })
            } else {
                panic!()
            },
            Value::Float(f1) => if let Value::Float(f2) = v2 {
                Value::Bool(match compare_type {
                    CompareType::EQ => f1 == f2,
                    _ => unreachable!(),
                })
            } else {
                panic!()
            },
            Value::Bool(b1) => if let Value::Bool(b2) = v2 {
                Value::Bool(match compare_type {
                    CompareType::EQ => b1 == b2,
                    _ => unreachable!(),
                })
            } else {
                panic!()
            },
            Value::String(s1) => if let Value::String(s2) = v2 {
                Value::Bool(match compare_type {
                    CompareType::EQ => s1 == s2,
                    _ => unreachable!(),
                })
            } else {
                panic!()
            },
            _ => panic!(),
        });
        self.advance();
    }

    fn call(&mut self) {}

    fn branch_if(&mut self, then_block: &usize, else_block: &usize) {
        match self.stack.pop().unwrap() {
            Value::Bool(true) => self.current_block = *then_block,
            Value::Bool(false) => self.current_block = *else_block,
            _ => panic!(),
        };
        self.current_instruction = 0;
    }

    fn jump(&mut self, block: &usize) {
        self.current_block = *block;
        self.current_instruction = 0;
    }

    fn get_function(&mut self, func: &usize) {}
}






