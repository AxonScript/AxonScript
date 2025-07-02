//semantic analysis, 
//it checks the AST, 
//checks whether we declared the variable before using it, 
//checks the types of variables, division by zero, consistency in math


use crate::ast::*;
use crate::high_level_ir::*;
use crate::semantic::semantic_error::SemanticError;
use std::collections::{HashMap, HashSet};

pub struct SemanticResult<T> {
    pub result: T,
    pub errors: Vec<SemanticError>,
    pub mutable_vars: HashSet<String>,
}

struct SemanticContext {
    functions: HashSet<String>,
    variables: HashMap<String, HIRType>,
    const_values: HashMap<String, i64>,
    mutable_vars: HashSet<String>,
    start_count: usize,
    loop_depth: usize,
}

pub fn ast_to_hir(ast: Vec<Statement>, src: Option<String>) -> SemanticResult<Vec<HIRStatement>> {
    let mut ctx = SemanticContext {
        functions: HashSet::new(),
        variables: HashMap::new(),
        const_values: HashMap::new(),
        mutable_vars: HashSet::new(),
        start_count: 0,
        loop_depth: 0,
    };
    let mut intermediate = ast_to_hir_with_ctx(ast, &src, &mut ctx);
    intermediate.result.insert(
        0,
        HIRStatement::Assignment {
            name: "Result".to_string(),
            value: HIRExpr::Int32(0),
        },
    );
    if ctx.start_count != 1 {
        intermediate.errors.push(SemanticError::new(
            format!(
                "\x1b[1;31m[ERR-SEM-301]\x1b[0m Program must have \x1b[1;33mexactly one\x1b[0m \x1b[1;36mcast Start() << >>\x1b[0m function.\n\
Hint: Declare a start function like so:\n\
\x1b[1;32mcast Start() >>\n   ?? ...your code...\n<<\x1b[0m"
            ),
            0,
            0,
            src.clone(),
        ));
    }
    SemanticResult {
        result: intermediate.result,
        errors: intermediate.errors,
        mutable_vars: ctx.mutable_vars.clone(),
    }
}

fn ast_to_hir_with_ctx(
    ast: Vec<Statement>,
    src: &Option<String>,
    ctx: &mut SemanticContext,
) -> SemanticResult<Vec<HIRStatement>> {
    let mut hir = Vec::new();
    let mut errors = Vec::new();
    for stmt in ast {
        let res = statement_to_hir(stmt, src, ctx);
        hir.extend(res.result);
        errors.extend(res.errors);
    }
    SemanticResult {
        result: hir,
        errors,
        mutable_vars: HashSet::new(),
    }
}

fn statement_to_hir(
    stmt: Statement,
    src: &Option<String>,
    ctx: &mut SemanticContext,
) -> SemanticResult<Vec<HIRStatement>> {
    let mut errors = Vec::new();
    let mut out = Vec::new();

    match stmt {
        Statement::Do(inner) => {
            let res = statement_to_hir(*inner, src, ctx);
            errors.extend(res.errors);
            out.extend(res.result);
        }
        Statement::Assignment {
            name,
            mutable,
            type_var,
            value,
        } => {
            if !mutable && ctx.variables.contains_key(&name) && ctx.mutable_vars.contains(&name) {
                errors.push(SemanticError::new(
                    format!(
                        "\x1b[1;31m[ERR-SEM-560]\x1b[0m Cannot reassign to immutable variable '{}'",
                        name
                    ),
                    0,
                    0,
                    src.clone(),
                ));
            }
            let value_res = expr_to_hir(value, src, ctx);
            errors.extend(value_res.errors);
            match &value_res.result {
                HIRExpr::Int32(v) => {
                    ctx.const_values.insert(name.clone(), *v as i64);
                }
                HIRExpr::Int64(v) => {
                    ctx.const_values.insert(name.clone(), *v);
                }
                _ => {
                    ctx.const_values.remove(&name);
                }
            }
            if let Some(t) = type_var {
                ctx.variables.insert(name.clone(), type_to_hir(t));
            }
            if mutable {
                ctx.mutable_vars.insert(name.clone());
            } else {
                ctx.mutable_vars.remove(&name);
            }
            out.push(HIRStatement::Assignment {
                name,
                value: value_res.result,
            });
        }
        Statement::FunctionCall {
            name,
            params,
            start,
            body,
        } => {
            if start {
                ctx.start_count += 1;
            }
            let mut hir_params = Vec::new();
            for param in params {
                if let Expr::Identifier(id) = param {
                    ctx.variables.insert(id.clone(), HIRType::Void);
                    hir_params.push((id, HIRType::Void));
                }
            }
            let body_res = ast_to_hir_with_ctx(body, src, ctx);
            errors.extend(body_res.errors);
            out.push(HIRStatement::Function {
                name,
                params: hir_params,
                return_type: HIRType::Void,
                start,
                body: body_res.result,
            });
        }
        Statement::Print { params } => {
            let mut hir_params = Vec::new();
            for expr in params {
                let res = expr_to_hir(expr, src, ctx);
                errors.extend(res.errors);
                hir_params.push(res.result);
            }
            out.push(HIRStatement::Print { params: hir_params });
        }

        Statement::Math { expression, destination, .. } => {
            let res = expr_to_hir(expression, src, ctx);
            errors.extend(res.errors);
            let ty = infer_expr_type(&res.result, ctx);
            ctx.variables.insert(destination.clone(), ty);
            out.push(HIRStatement::Assignment {
                name: destination,
                value: res.result,
            });
        }

        Statement::Input { target, .. } => {

            let res = expr_to_hir(target, src, ctx);
            errors.extend(res.errors);
            out.push(HIRStatement::Input { target: res.result });
        }
        Statement::If { logic, args, body, else_body } => {
            if args.len() == 2 {
                let left_res = expr_to_hir(args[0].clone(), src, ctx);
                let right_res = expr_to_hir(args[1].clone(), src, ctx);
                errors.extend(left_res.errors);
                errors.extend(right_res.errors);
                let op = logic_to_hir(logic);
                let cond = HIRExpr::BinaryOp {
                    left: Box::new(left_res.result),
                    op,
                    right: Box::new(right_res.result),
                };
                let body_res = ast_to_hir_with_ctx(body, src, ctx);
                errors.extend(body_res.errors);
                let else_hir = match else_body {
                    Some(b) => {
                        let else_res = ast_to_hir_with_ctx(b, src, ctx);
                        errors.extend(else_res.errors);
                        Some(else_res.result)
                    }
                    None => None,
                };
                out.push(HIRStatement::If {
                    condition: cond,
                    body: body_res.result,
                    else_body: else_hir,
                });
            }
        }
        Statement::Loop { body } => {
            ctx.loop_depth += 1;
            let body_res = ast_to_hir_with_ctx(body, src, ctx);
            ctx.loop_depth -= 1;
            errors.extend(body_res.errors);
            out.push(HIRStatement::Loop { body: body_res.result });
        }
        Statement::While { logic, args, body } => {
            if args.len() == 2 {
                let left_res = expr_to_hir(args[0].clone(), src, ctx);
                let right_res = expr_to_hir(args[1].clone(), src, ctx);
                errors.extend(left_res.errors);
                errors.extend(right_res.errors);
                let op = logic_to_hir(logic);
                let cond = HIRExpr::BinaryOp {
                    left: Box::new(left_res.result),
                    op,
                    right: Box::new(right_res.result),
                };
                ctx.loop_depth += 1;
                let body_res = ast_to_hir_with_ctx(body, src, ctx);
                ctx.loop_depth -= 1;
                errors.extend(body_res.errors);
                out.push(HIRStatement::While {
                    condition: cond,
                    body: body_res.result,
                });
            }
        }
        Statement::Break => {
            if ctx.loop_depth == 0 {
                errors.push(SemanticError::new(
                    "\x1b[1;31m[ERR-SEM-310]\x1b[0m 'break' used outside of loop",
                    0,
                    0,
                    src.clone(),
                ));
            } else {
                out.push(HIRStatement::Break);
            }
        }
    }
    SemanticResult {
        result: out,
        errors,
        mutable_vars: HashSet::new(),
    }
}

fn expr_to_hir(expr: Expr, src: &Option<String>, ctx: &SemanticContext) -> SemanticResult<HIRExpr> {
    let mut errors = Vec::new();
    let result = match expr {
        Expr::Int32(i) => HIRExpr::Int32(i),
        Expr::Float32(f) => HIRExpr::Float32(f),
        Expr::Int64(i) => HIRExpr::Int64(i),
        Expr::Float64(f) => HIRExpr::Float64(f),
        Expr::String(s) => HIRExpr::String(s),
        Expr::Bool(b) => HIRExpr::Bool(b),
        Expr::Identifier(name) => {
            if !ctx.variables.contains_key(&name) {
                errors.push(SemanticError::new(
                    format!(
                        "\x1b[1;31m[ERR-SEM-999]\x1b[0m Variable '{}' used before declaration",
                        name
                    ),
                    0,
                    0,
                    src.clone(),
                ));
            }
            HIRExpr::Identifier(name)
        }
        Expr::BinaryOp { left, op, right } => {
            if let Operator::Divide = op {
                if let Expr::Identifier(name) = &*right {
                    if let Some(val) = ctx.const_values.get(name) {
                        if *val == 0 {
                            errors.push(SemanticError::new(
                                format!(
                                    "\x1b[1;31m[ERR-SEM-550]\x1b[0m Division by variable '{}' with known value 0",
                                    name
                                ),
                                0,
                                0,
                                src.clone(),
                            ));
                        }
                    }
                }
            }
            let left_res = expr_to_hir(*left, src, ctx);
            let right_res = expr_to_hir(*right, src, ctx);
            errors.extend(left_res.errors);
            errors.extend(right_res.errors);
            let left_ty = infer_expr_type(&left_res.result, ctx);
            let right_ty = infer_expr_type(&right_res.result, ctx);
            match coerce_types(left_res.result, left_ty, right_res.result, right_ty) {
                Ok((new_left, new_right, _)) => HIRExpr::BinaryOp {
                    left: Box::new(new_left),
                    op: operator_to_hir(op),
                    right: Box::new(new_right),
                },
                Err(e) => {
                    errors.push(SemanticError::new(e, 0, 0, src.clone()));
                    HIRExpr::Int32(0)
                }
            }
        }
        Expr::Vector(_) => {
            errors.push(SemanticError::new(
                "\x1b[1;31m[ERR-SEM-230]\x1b[0m Vector literal expressions are not directly supported in HIRExpr.",
                0,
                0,
                src.clone(),
            ));
            HIRExpr::Int32(0)
        }
    };
    SemanticResult {
        result,
        errors,
        mutable_vars: HashSet::new(),
    }
}

fn infer_expr_type(expr: &HIRExpr, ctx: &SemanticContext) -> HIRType {
    match expr {
        HIRExpr::Int32(_) => HIRType::I32,
        HIRExpr::Int64(_) => HIRType::I64,
        HIRExpr::Float32(_) => HIRType::F32,
        HIRExpr::Float64(_) => HIRType::F64,
        HIRExpr::String(_) => HIRType::String,
        HIRExpr::Bool(_) => HIRType::Bool,
        HIRExpr::Identifier(name) => ctx.variables.get(name).cloned().unwrap_or(HIRType::Void),
        HIRExpr::BinaryOp { left, op, right } => {
            let lt = infer_expr_type(left, ctx);
            let rt = infer_expr_type(right, ctx);
            let (_, _, common_ty) =
                coerce_types((**left).clone(), lt, (**right).clone(), rt).unwrap_or((left.as_ref().clone(), right.as_ref().clone(), HIRType::Void));
            match op {
                HIROperator::Equals
                | HIROperator::NotEquals
                | HIROperator::GreaterThan
                | HIROperator::LessThan
                | HIROperator::GreaterEqual
                | HIROperator::LessEqual => HIRType::Bool,
                _ => common_ty,
            }
        }
        HIRExpr::FunctionCall { .. } => HIRType::Void,
        HIRExpr::Coerce { target, .. } => target.clone(),
    }
}

fn type_to_hir(typ: Type) -> HIRType {
    match typ {
        Type::I32 => HIRType::I32,
        Type::I64 => HIRType::I64,
        Type::F32 => HIRType::F32,
        Type::F64 => HIRType::F64,
        Type::String => HIRType::String,
        Type::Bool => HIRType::Bool,
        Type::Vector(_) => panic!("Vector types are not supported in HIRType."),
    }
}

fn operator_to_hir(op: Operator) -> HIROperator {
    match op {
        Operator::Plus => HIROperator::Plus,
        Operator::Minus => HIROperator::Minus,
        Operator::Multiply => HIROperator::Multiply,
        Operator::Divide => HIROperator::Divide,
    }
}

fn logic_to_hir(logic: Logic) -> HIROperator {
    match logic {
        Logic::Equal => HIROperator::Equals,
        Logic::NotEqual => HIROperator::NotEquals,
        Logic::Greater => HIROperator::GreaterThan,
        Logic::Less => HIROperator::LessThan,
        Logic::GreaterEqual => HIROperator::GreaterEqual,
        Logic::LessEqual => HIROperator::LessEqual,
    }
}

fn coerce_types(
    left: HIRExpr,
    left_ty: HIRType,
    right: HIRExpr,
    right_ty: HIRType,
) -> Result<(HIRExpr, HIRExpr, HIRType), String> {
    use HIRType::*;
    match (&left_ty, &right_ty) {
        (a, b) if a == b => Ok((left, right, a.clone())),
        (I32, I64) | (I64, I32) => Ok((
            if left_ty == I32 {
                HIRExpr::Coerce {
                    expr: Box::new(left),
                    target: I64,
                }
            } else {
                left
            },
            if right_ty == I32 {
                HIRExpr::Coerce {
                    expr: Box::new(right),
                    target: I64,
                }
            } else {
                right
            },
            I64,
        )),
        (I32, F64) | (F64, I32) | (I64, F64) | (F64, I64) | (F32, F64) | (F64, F32) => Ok((
            if left_ty != F64 {
                HIRExpr::Coerce {
                    expr: Box::new(left),
                    target: F64,
                }
            } else {
                left
            },
            if right_ty != F64 {
                HIRExpr::Coerce {
                    expr: Box::new(right),
                    target: F64,
                }
            } else {
                right
            },
            F64,
        )),
        (Bool, I32) | (I32, Bool) => Ok((
            if left_ty == Bool {
                HIRExpr::Coerce {
                    expr: Box::new(left),
                    target: I32,
                }
            } else {
                left
            },
            if right_ty == Bool {
                HIRExpr::Coerce {
                    expr: Box::new(right),
                    target: I32,
                }
            } else {
                right
            },
            I32,
        )),
        _ => Err(format!(
            "Cannot coerce types: {:?} and {:?}",
            left_ty, right_ty
        )),
    }
}
