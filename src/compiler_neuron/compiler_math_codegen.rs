//IR generation for mathematical operations


use super::compiler_context::Compiler;
use crate::high_level_ir::{HIRExpr, HIROperator, HIRType};
use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::*;
use llvm_sys::prelude::LLVMValueRef;

pub fn codegen_math_expr(
    compiler: &mut Compiler,
    left: &HIRExpr,
    op: &HIROperator,
    right: &HIRExpr,
    global_var_name: Option<&str>,
) -> Result<(LLVMValueRef, HIRType), String> {
    unsafe {
        let (mut left_val, mut left_ty) = compiler.codegen_expr(left)?;
        let (mut right_val, mut right_ty) = compiler.codegen_expr(right)?;
        if left_val.is_null() || right_val.is_null() {
            return Err("\x1b[31m[ERR-SEM-695] Null operand value\x1b[0m".to_string());
        }
        if left_ty != right_ty {
            match (left_ty.clone(), right_ty.clone()) {
                (HIRType::I32, HIRType::I64) => {
                    left_val = LLVMBuildSExt(
                        compiler.builder,
                        left_val,
                        compiler.hir_type_to_llvm_type(&HIRType::I64),
                        b"sext\0".as_ptr() as _,
                    );
                    left_ty = HIRType::I64;
                }
                (HIRType::I64, HIRType::I32) => {
                    right_val = LLVMBuildSExt(
                        compiler.builder,
                        right_val,
                        compiler.hir_type_to_llvm_type(&HIRType::I64),
                        b"sext\0".as_ptr() as _,
                    );
                    right_ty = HIRType::I64;
                }
                (HIRType::F32, HIRType::F64) => {
                    left_val = LLVMBuildFPExt(
                        compiler.builder,
                        left_val,
                        compiler.hir_type_to_llvm_type(&HIRType::F64),
                        b"fpext\0".as_ptr() as _,
                    );
                    left_ty = HIRType::F64;
                }
                (HIRType::F64, HIRType::F32) => {
                    right_val = LLVMBuildFPExt(
                        compiler.builder,
                        right_val,
                        compiler.hir_type_to_llvm_type(&HIRType::F64),
                        b"fpext\0".as_ptr() as _,
                    );
                    right_ty = HIRType::F64;
                }
                _ => {
                    return Err(format!(
                        "\x1b[31m[ERR-SEM-510] Type mismatch: {:?} vs {:?}\x1b[0m",
                        left_ty, right_ty
                    ));
                }
            }
        }
        let is_float = matches!(left_ty, HIRType::F32 | HIRType::F64);
        let result_type = if matches!(
            op,
            HIROperator::Equals
                | HIROperator::NotEquals
                | HIROperator::GreaterThan
                | HIROperator::LessThan
                | HIROperator::GreaterEqual
                | HIROperator::LessEqual
        ) {
            HIRType::Bool
        } else {
            left_ty.clone()
        };
        let result = match op {
            HIROperator::Plus => {
                if is_float {
                    LLVMBuildFAdd(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"fadd\0".as_ptr() as *const _,
                    )
                } else {
                    LLVMBuildAdd(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"add\0".as_ptr() as *const _,
                    )
                }
            }
            HIROperator::Minus => {
                if is_float {
                    LLVMBuildFSub(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"fsub\0".as_ptr() as *const _,
                    )
                } else {
                    LLVMBuildSub(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"sub\0".as_ptr() as *const _,
                    )
                }
            }
            HIROperator::Multiply => {
                if is_float {
                    LLVMBuildFMul(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"fmul\0".as_ptr() as *const _,
                    )
                } else {
                    LLVMBuildMul(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"mul\0".as_ptr() as *const _,
                    )
                }
            }
            HIROperator::Divide => {
                if is_float {
                    match right {
                        HIRExpr::Float32(0.0) | HIRExpr::Float64(0.0) => {
                            return Err("\x1b[31m[ERR-SEM-550] Division by zero\x1b[0m".to_string());
                        }
                        _ => {}
                    }
                    LLVMBuildFDiv(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"fdiv\0".as_ptr() as *const _,
                    )
                } else {
                    match right {
                        HIRExpr::Int32(0) | HIRExpr::Int64(0) => {
                            return Err("\x1b[31m[ERR-SEM-550] Division by zero\x1b[0m".to_string());
                        }
                        _ => {}
                    }
                    LLVMBuildSDiv(
                        compiler.builder,
                        left_val,
                        right_val,
                        b"sdiv\0".as_ptr() as *const _,
                    )
                }
            }
            cmp_op => {
                if is_float {
                    let float_predicate = match cmp_op {
                        HIROperator::Equals => llvm_sys::LLVMRealPredicate::LLVMRealOEQ,
                        HIROperator::NotEquals => llvm_sys::LLVMRealPredicate::LLVMRealONE,
                        HIROperator::GreaterThan => llvm_sys::LLVMRealPredicate::LLVMRealOGT,
                        HIROperator::LessThan => llvm_sys::LLVMRealPredicate::LLVMRealOLT,
                        HIROperator::GreaterEqual => llvm_sys::LLVMRealPredicate::LLVMRealOGE,
                        HIROperator::LessEqual => llvm_sys::LLVMRealPredicate::LLVMRealOLE,
                        _ => unreachable!(),
                    };
                    LLVMBuildFCmp(
                        compiler.builder,
                        float_predicate,
                        left_val,
                        right_val,
                        b"fcmp\0".as_ptr() as *const _,
                    )
                } else {
                    let int_predicate = match cmp_op {
                        HIROperator::Equals => LLVMIntPredicate::LLVMIntEQ,
                        HIROperator::NotEquals => LLVMIntPredicate::LLVMIntNE,
                        HIROperator::GreaterThan => LLVMIntPredicate::LLVMIntSGT,
                        HIROperator::LessThan => LLVMIntPredicate::LLVMIntSLT,
                        HIROperator::GreaterEqual => LLVMIntPredicate::LLVMIntSGE,
                        HIROperator::LessEqual => LLVMIntPredicate::LLVMIntSLE,
                        _ => unreachable!(),
                    };
                    LLVMBuildICmp(
                        compiler.builder,
                        int_predicate,
                        left_val,
                        right_val,
                        b"icmp\0".as_ptr() as *const _,
                    )
                }
            }
        };
        if result.is_null() {
            return Err("\x1b[31m[ERR-SEM-696] Failed to generate math instruction\x1b[0m".to_string());
        }
        if let Some(var_name) = global_var_name {
            if compiler.current_function.is_none() {
                return Err("\x1b[31m[ERR-SEM-697] Cannot store to global outside function\x1b[0m".to_string());
            }
            let current_block = LLVMGetInsertBlock(compiler.builder);
            if current_block.is_null() {
                return Err("\x1b[31m[ERR-SEM-670] No basic block set\x1b[0m".to_string());
            }
            let (var_ptr, var_type) = compiler
                .variables
                .get(var_name)
                .ok_or_else(|| format!("\x1b[31m[ERR-SEM-698] Global variable {} not found\x1b[0m", var_name))?;
            if *var_type != result_type {
                return Err(format!(
                    "\x1b[31m[ERR-SEM-510] Type mismatch: global var {:?} vs result {:?}\x1b[0m",
                    var_type, result_type
                ));
            }
            let store = LLVMBuildStore(compiler.builder, result, *var_ptr);
            if store.is_null() {
                return Err("\x1b[31m[ERR-SEM-694] Failed to store value\x1b[0m".to_string());
            }
        }
        Ok((result, result_type))
    }
}