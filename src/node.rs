use std::boxed::Box;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    StatementList(Vec<Node>),
    VarDecl(String, Option<Box<Node>>),
    Member(Box<Node>, String),
    Call(Box<Node>, Vec<Node>),
    If(Box<Node>, Box<Node>, Box<Node>), // Cond, Then, Else
    While(Box<Node>, Box<Node>),         // Cond, Body
    Assign(Box<Node>, Box<Node>),
    UnaryOp(Box<Node>, UnaryOp),
    BinaryOp(Box<Node>, Box<Node>, BinOp),
    TernaryOp(Box<Node>, Box<Node>, Box<Node>),
    Identifier(String),
    String(String),
    Boolean(bool),
    Number(f64),
    Nope,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
    Delete,
    Void,
    Typeof,
    Plus,
    Minus,
    BitwiseNot,
    Not,
    PrInc, // Prefix
    PrDec,
    PoInc, // Postfix
    PoDec,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
    And,
    Or,
    Xor,
    LAnd,
    LOr,
    Eq,
    Ne,
    SEq, // Strict Eq
    SNe, // Strict Ne
    Lt,
    Gt,
    Le,
    Ge,
    Shl,
    Shr,
    ZFShr,
    Comma,
    Assign,
}
