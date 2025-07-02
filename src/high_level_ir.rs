//HIR(high level IR) is an AST that has already undergone semantic analysis, 
//and it is also lower-level

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HIRType {
    I32,
    I64,
    F32,
    F64,
    String,
    Bool,
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HIROperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HIRExpr {
    Int32(i32),
    Float32(f32),
    Int64(i64),
    Float64(f64),
    String(String),
    Bool(bool),
    Identifier(String),
    BinaryOp {
        left: Box<HIRExpr>,
        op: HIROperator,
        right: Box<HIRExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<HIRExpr>,
    },
    Coerce {
        expr: Box<HIRExpr>,
        target: HIRType,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HIRStatement {
    Assignment {
        name: String,
        value: HIRExpr,
    },
    Function {
        name: String,
        params: Vec<(String, HIRType)>,
        return_type: HIRType,
        start: bool,
        body: Vec<HIRStatement>,
    },
    Print {
        params: Vec<HIRExpr>,
    },
    ExprStatement {
        expr: HIRExpr,
    },
    If {
        condition: HIRExpr,
        body: Vec<HIRStatement>,
        else_body: Option<Vec<HIRStatement>>,
    },
    Loop {
        body: Vec<HIRStatement>,
    },
    While {
        condition: HIRExpr,
        body: Vec<HIRStatement>,
    },
    Break,
    Input {
        target: HIRExpr,
    },
}
