//AST (abstract syntax tree) is what the code turns into after parsing
//its a tree structure that shows the syntactic structure of the program

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    String,
    Bool,
    Vector(Box<Type>),
}
#[derive(Debug, Clone)]
pub enum Expr {
    Int32(i32),
    Float32(f32),
    Int64(i64),
    Float64(f64),
    String(String),
    Vector(Vec<Expr>),
    Bool(bool),
    Identifier(String),
    BinaryOp {
        left: Box<Expr>,
        op: Operator,
        right: Box<Expr>,
    },
}
#[derive(Debug, Clone)]
pub enum Statement {
    Assignment {
        name: String,
        mutable: bool,
        type_var: Option<Type>,
        value: Expr,
    },
    FunctionCall {
        name: String,
        params: Vec<Expr>,
        start: bool,
        body: Vec<Statement>,
    },
    Print {
        params: Vec<Expr>,
    },

    Do(Box<Statement>),
    Math {
        expression: Expr,
        destination: String,
        err: Option<String>,
    },
    If {
        logic: Logic,
        args: Vec<Expr>,
        body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    Loop {
        body: Vec<Statement>,
    },
    While {
        logic: Logic,
        args: Vec<Expr>,
        body: Vec<Statement>,
    },
    Break,
    Input {
        target: Expr,
        err: Option<String>,
    },
}
#[derive(Debug, Clone, Copy)]
pub enum Logic {
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
}
#[derive(Debug, Clone, Copy)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
}
