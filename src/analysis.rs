use bytecode_gen::{ByteCode, ByteCodeGenerator, VMInst};
use gc::MemoryAllocator;
use id::IdGen;
use node::{
    BinOp, FormalParameter, FormalParameters, IdentifierInfo, MethodDefinitionKind, Node, NodeBase,
    PropertyDefinition, UnaryOp, VarKind,
};
use parser::Error;
use rustc_hash::{FxHashMap, FxHashSet};
use vm::constant::{ConstantTable, SpecialProperties, SpecialPropertyKind};
use vm::jsvalue::function::{DestinationKind, Exception};
use vm::jsvalue::value::Value2;
use vm::jsvalue::{prototype, value};

#[derive(Clone, Debug)]
pub enum Level {
    Function { params: VarMap, varmap: VarMap },
    Block { varmap: VarMap },
}

#[derive(Debug)]
pub struct Analyzer {
    level: Vec<Level>,
    idgen: IdGen,
}

pub type VarMap = FxHashMap<String, Option<usize>>;

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            level: vec![Level::Function {
                params: VarMap::default(),
                varmap: VarMap::default(),
            }],
            idgen: IdGen::new(),
        }
    }

    pub fn analyze(&mut self, node: Node) -> Result<Node, Error> {
        if let NodeBase::StatementList(stmts) = node.base {
            let mut new_stmts = vec![];
            for stmt in stmts {
                new_stmts.push(self.visit(stmt)?)
            }
            Ok(Node::new(NodeBase::StatementList(new_stmts), node.pos))
        } else {
            panic!()
        }
    }
}

impl Analyzer {
    fn collect_variable_declarations(&mut self, node: &Node) {
        let stmts = if let NodeBase::StatementList(ref stmts) = node.base {
            stmts
        } else {
            return;
        };
        let varmap = self.get_current_varmap_mut();
        for stmt in stmts {
            if let NodeBase::VarDecl(ref name, _, _) = stmt.base {
                varmap.insert(name.clone(), None);
            }
        }
    }

    fn replace_variable_declarations(
        &mut self,
        node: &mut Node,
        bound_variables: &FxHashMap<String, usize>,
    ) {
        let stmts = if let NodeBase::StatementList(ref mut stmts) = node.base {
            stmts
        } else {
            return;
        };
        for stmt in stmts {
            if let NodeBase::VarDecl(name, init, _) = stmt.base.clone() {
                if let Some(offset) = bound_variables.get(&name) {
                    if let Some(init) = init {
                        stmt.base = NodeBase::Assign(
                            Box::new(Node::new(
                                NodeBase::Identifier(IdentifierInfo::Offset(*offset)),
                                0,
                            )),
                            init,
                        );
                    }
                }
            }
        }
    }

    fn visit(&mut self, node: Node) -> Result<Node, Error> {
        match node.base {
            NodeBase::StatementList(stmts) => {
                for stmt in &stmts {
                    self.collect_variable_declarations(stmt);
                }

                let mut new_stmts = vec![];
                for stmt in stmts {
                    new_stmts.push(self.visit(stmt)?)
                }

                let mut bound_variables = FxHashMap::default();
                for (name, offset) in self.get_current_varmap() {
                    if let Some(offset) = offset {
                        bound_variables.insert(name.clone(), *offset);
                    }
                }

                for stmt in &mut new_stmts {
                    self.replace_variable_declarations(stmt, &bound_variables);
                }

                Ok(Node::new(NodeBase::StatementList(new_stmts), node.pos))
            }
            NodeBase::Block(stmts) => {
                self.push_new_block_level();

                for stmt in &stmts {
                    self.collect_variable_declarations(stmt);
                }

                let mut new_stmts = vec![];
                for stmt in stmts {
                    new_stmts.push(self.visit(stmt)?)
                }

                let level = self.pop_level();
                let varmap = level.get_varmap();
                let mut bound_variables: FxHashMap<String, usize> = FxHashMap::default();
                for (name, offset) in varmap {
                    if let Some(offset) = offset {
                        bound_variables.insert(name.clone(), *offset);
                    }
                }

                for stmt in &mut new_stmts {
                    self.replace_variable_declarations(stmt, &bound_variables);
                }

                Ok(Node::new(NodeBase::Block(new_stmts), node.pos))
            }
            NodeBase::If(cond, then, else_) => Ok(Node::new(
                NodeBase::If(
                    Box::new(self.visit(*cond)?),
                    Box::new(self.visit(*then)?),
                    Box::new(self.visit(*else_)?),
                ),
                node.pos,
            )),
            NodeBase::While(cond, body) => Ok(Node::new(
                NodeBase::While(Box::new(self.visit(*cond)?), Box::new(self.visit(*body)?)),
                node.pos,
            )),
            NodeBase::For(init, cond, step, body) => Ok(Node::new(
                NodeBase::For(
                    Box::new(self.visit(*init)?),
                    Box::new(self.visit(*cond)?),
                    Box::new(self.visit(*step)?),
                    Box::new(self.visit(*body)?),
                ),
                node.pos,
            )),
            NodeBase::Break(_) => Ok(node),
            NodeBase::Try(try, catch, param, finally) => Ok(Node::new(
                NodeBase::Try(
                    Box::new(self.visit(*try)?),
                    Box::new(self.visit(*catch)?),
                    Box::new(self.visit(*param)?),
                    Box::new(self.visit(*finally)?),
                ),
                node.pos,
            )),
            NodeBase::FunctionDecl {
                name,
                mut params,
                body,
                ..
            } => {
                self.idgen.save();
                let mut params_varmap = VarMap::default();
                for FormalParameter { ref name, .. } in &params {
                    params_varmap.insert(name.clone(), None);
                }
                self.push_new_function_level(params_varmap);

                let body = Box::new(self.visit(*body)?);
                let (params_varmap, varmap) = self.pop_level().as_function_level();

                for FormalParameter {
                    ref name,
                    ref mut bound,
                    ..
                } in &mut params
                {
                    if let Some(offset) = params_varmap.get(name).unwrap() {
                        *bound = Some(*offset);
                    }
                }

                self.idgen.restore();

                Ok(Node::new(
                    NodeBase::FunctionDecl {
                        name,
                        params,
                        body,
                        bound_variables: self.idgen.get_cur_id(),
                    },
                    node.pos,
                ))
            } // NodeBase::FunctionExpr(ref name, ref params, ref body) => {
            //     self.visit_function_expr(name, params, &*body, true, iseq, use_value)?
            // }
            // NodeBase::ArrowFunction(ref params, ref body) => {
            //     self.visit_function_expr(&None, params, &*body, false, iseq, use_value)?
            // }
            NodeBase::VarDecl(name, init, kind) => Ok(Node::new(
                NodeBase::VarDecl(
                    name,
                    if let Some(init) = init {
                        Some(Box::new(self.visit(*init)?))
                    } else {
                        None
                    },
                    kind,
                ),
                node.pos,
            )),
            // NodeBase::Member(ref parent, ref property) => {
            //     self.visit_member(&*parent, property, iseq, use_value)?
            // }
            // NodeBase::Index(ref parent, ref index) => {
            //     self.visit_index(&*parent, &*index, iseq, use_value)?
            // }
            // NodeBase::UnaryOp(ref expr, ref op) => {
            //     self.visit_unary_op(&*expr, op, iseq, use_value)?
            // }
            // NodeBase::BinaryOp(ref lhs, ref rhs, ref op) => {
            //     self.visit_binary_op(&*lhs, &*rhs, op, iseq, use_value)?
            // }
            // NodeBase::Assign(ref dst, ref src) => {
            //     self.visit_assign(&*dst, &*src, iseq, use_value)?
            // }
            // NodeBase::Call(ref callee, ref args) => {
            //     self.visit_call(&*callee, args, iseq, use_value)?
            // }
            // NodeBase::Throw(ref val) => self.visit_throw(val, iseq)?,
            // NodeBase::Return(ref val) => self.visit_return(val, iseq)?,
            // NodeBase::New(ref expr) => self.visit_new(&*expr, iseq, use_value)?,
            // NodeBase::Object(ref properties) => self.visit_object_literal(properties, iseq)?,
            // NodeBase::Array(ref elems) => self.visit_array_literal(elems, iseq)?,
            NodeBase::Identifier(info) => {
                let name = info.get_name();
                Ok(Node::new(
                    NodeBase::Identifier(if let Some(offset) = self.use_variable(&name) {
                        IdentifierInfo::Offset(offset)
                    } else {
                        IdentifierInfo::Name(name)
                    }),
                    node.pos,
                ))
            }
            // // NodeBase::Undefined => {
            // //     if use_value {
            // //         self.bytecode_generator.append_push_undefined(iseq);
            // //     }
            // // }
            // NodeBase::Null => {
            //     if use_value {
            //         self.bytecode_generator.append_push_null(iseq);
            //     }
            // }
            // NodeBase::This => {
            //     if use_value {
            //         self.bytecode_generator.append_push_this(iseq);
            //     }
            // }
            // NodeBase::String(ref s) => {
            //     if use_value {
            //         self.bytecode_generator
            //             .append_push_const(Value2::string(self.memory_allocator, s.clone()), iseq)
            //     }
            // }
            // NodeBase::Number(n) => {
            //     if use_value {
            //         self.bytecode_generator.append_push_number(n, iseq)
            //     }
            // }
            // NodeBase::Boolean(b) => {
            //     if use_value {
            //         self.bytecode_generator.append_push_bool(b, iseq)
            //     }
            // }
            // NodeBase::Nope => {
            //     if use_value {
            //         self.bytecode_generator
            //             .append_push_const(Value2::empty(), iseq)
            //     }
            // }
            // ref e => unimplemented!("{:?}", e),
            _ => Ok(node),
        }
    }
}

impl Analyzer {
    pub fn push_new_block_level(&mut self) {
        self.level.push(Level::Block {
            varmap: VarMap::default(),
        })
    }

    pub fn push_new_function_level(&mut self, params: VarMap) {
        self.level.push(Level::Function {
            params,
            varmap: VarMap::default(),
        })
    }

    pub fn pop_level(&mut self) -> Level {
        self.level.pop().unwrap()
    }

    pub fn get_current_varmap_mut(&mut self) -> &mut VarMap {
        match self.level.last_mut().unwrap() {
            Level::Function { ref mut varmap, .. } => varmap,
            Level::Block { ref mut varmap } => varmap,
        }
    }

    pub fn get_current_varmap(&self) -> &VarMap {
        match self.level.last().unwrap() {
            Level::Function { ref varmap, .. } => varmap,
            Level::Block { ref varmap } => varmap,
        }
    }

    pub fn use_variable(&mut self, name: &String) -> Option<usize> {
        for level in self.level.iter_mut().rev() {
            match level {
                Level::Function {
                    ref mut varmap,
                    ref mut params,
                } => {
                    if let Some(v) = params.get_mut(name) {
                        if v.is_some() {
                            return *v;
                        }
                        *v = Some(self.idgen.gen_id());
                        return *v;
                    }

                    if let Some(v) = varmap.get_mut(name) {
                        if v.is_some() {
                            return *v;
                        }
                        *v = Some(self.idgen.gen_id());
                        return *v;
                    }

                    return None;
                }
                Level::Block { ref mut varmap } => {
                    if let Some(v) = varmap.get_mut(name) {
                        if v.is_some() {
                            return *v;
                        }
                        *v = Some(self.idgen.gen_id());
                        return *v;
                    }
                }
            }
        }

        None
    }
}

impl Level {
    pub fn get_varmap(&self) -> &VarMap {
        match self {
            Level::Function { ref varmap, .. } => varmap,
            Level::Block { ref varmap } => varmap,
        }
    }

    pub fn as_function_level(self) -> (VarMap, VarMap) {
        match self {
            Level::Function { params, varmap } => (params, varmap),
            _ => panic!(),
        }
    }
}