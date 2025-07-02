//llvm ir generation for conditions

use super::{codegen_statement, compiler_context::Compiler};
use crate::high_level_ir::{HIRStatement, HIRType};
use llvm_sys::core::*;
use llvm_sys::prelude::LLVMValueRef;
use llvm_sys::{LLVMIntPredicate, LLVMRealPredicate};

pub fn codegen_if(c: &mut Compiler, stmt: &HIRStatement) -> Result<(), String> {
    unsafe {
        let fun = c.current_function.ok_or("[ERR-SEM-524]")?;

        let HIRStatement::If { condition, body, else_body } = stmt else {
            return Err("[ERR-SEM-521]".into());
        };

        let (cond_val, ty) = c.codegen_expr(condition)?;

        let bool_cond = match ty {
            HIRType::Bool => cond_val,
            HIRType::I32 | HIRType::I64 => {
                let zero = LLVMConstInt(LLVMTypeOf(cond_val), 0, 0);
                LLVMBuildICmp(
                    c.builder,
                    LLVMIntPredicate::LLVMIntNE,
                    cond_val,
                    zero,
                    b"bool_cast\0".as_ptr() as _,
                )
            }
            HIRType::F32 | HIRType::F64 => {
                let zero = LLVMConstReal(LLVMTypeOf(cond_val), 0.0);
                LLVMBuildFCmp(
                    c.builder,
                    LLVMRealPredicate::LLVMRealONE,
                    cond_val,
                    zero,
                    b"bool_cast\0".as_ptr() as _,
                )
            }
            _ => return Err("[ERR-SEM-522] Unsupported type in if condition".into()),
        };

        let then_bb = LLVMAppendBasicBlockInContext(c.context, fun, b"if.then\0".as_ptr() as _);
        let merge_bb = LLVMAppendBasicBlockInContext(c.context, fun, b"if.merge\0".as_ptr() as _);
        let else_bb = if else_body.is_some() {
            LLVMAppendBasicBlockInContext(c.context, fun, b"if.else\0".as_ptr() as _)
        } else {
            merge_bb
        };

        LLVMBuildCondBr(c.builder, bool_cond, then_bb, else_bb);

        LLVMPositionBuilderAtEnd(c.builder, then_bb);
        for s in body {
            codegen_statement(c, s)?;
            // Если внутри тела был return/break/continue и блок завершён, выходим из цикла
            let then_block = LLVMGetInsertBlock(c.builder);
            if !LLVMGetBasicBlockTerminator(then_block).is_null() {
                break;
            }
        }
        let then_block = LLVMGetInsertBlock(c.builder);
        if LLVMGetBasicBlockTerminator(then_block).is_null() {
            LLVMBuildBr(c.builder, merge_bb);
        }

        if let Some(eb) = else_body {
            LLVMPositionBuilderAtEnd(c.builder, else_bb);
            for s in eb {
                codegen_statement(c, s)?;
                let else_block = LLVMGetInsertBlock(c.builder);
                if !LLVMGetBasicBlockTerminator(else_block).is_null() {
                    break;
                }
            }
            let else_block = LLVMGetInsertBlock(c.builder);
            if LLVMGetBasicBlockTerminator(else_block).is_null() {
                LLVMBuildBr(c.builder, merge_bb);
            }
        }

        LLVMPositionBuilderAtEnd(c.builder, merge_bb);
        Ok(())
    }
}