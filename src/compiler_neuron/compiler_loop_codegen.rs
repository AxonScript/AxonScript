//llvm ir generation for loops
use super::compiler_context::Compiler;
use crate::high_level_ir::{HIRStatement, HIRType};
use llvm_sys::core::*;

pub fn codegen_loop(compiler: &mut Compiler, stmt: &HIRStatement) -> Result<(), String> {
    unsafe {
        let current_func = compiler.current_function.ok_or("[ERR-SEM-532] No active function")?;
        let loop_header_bb = LLVMAppendBasicBlockInContext(
            compiler.context,
            current_func,
            b"loop_header\0".as_ptr() as _,
        );
        let loop_body_bb = LLVMAppendBasicBlockInContext(
            compiler.context,
            current_func,
            b"loop_body\0".as_ptr() as _,
        );
        let loop_exit_bb = LLVMAppendBasicBlockInContext(
            compiler.context,
            current_func,
            b"loop_exit\0".as_ptr() as _,
        );

        LLVMBuildBr(compiler.builder, loop_header_bb);
        LLVMPositionBuilderAtEnd(compiler.builder, loop_header_bb);

        compiler.break_targets.push(loop_exit_bb);

        LLVMBuildBr(compiler.builder, loop_body_bb);
        LLVMPositionBuilderAtEnd(compiler.builder, loop_body_bb);

        if let HIRStatement::Loop { body } = stmt {
            for s in body {
                super::codegen_statement(compiler, s)?;
                if !LLVMGetBasicBlockTerminator(LLVMGetInsertBlock(compiler.builder)).is_null() {
                    break;
                }
            }
        } else {
            return Err("[ERR-SEM-530] codegen_loop expected HIRStatement::Loop".into());
        }

        if LLVMGetBasicBlockTerminator(LLVMGetInsertBlock(compiler.builder)).is_null() {
            LLVMBuildBr(compiler.builder, loop_header_bb);
        }

        compiler.break_targets.pop();

        LLVMPositionBuilderAtEnd(compiler.builder, loop_exit_bb);

        Ok(())
    }
}

pub fn codegen_break(compiler: &mut Compiler) -> Result<(), String> {
    unsafe {
        let target = compiler.break_targets.last().copied().ok_or("[ERR-SEM-531] No loop context for break")?;
        if LLVMGetBasicBlockTerminator(LLVMGetInsertBlock(compiler.builder)).is_null() {
            LLVMBuildBr(compiler.builder, target);
        }
        Ok(())
    }
}

pub fn codegen_while(compiler: &mut Compiler, stmt: &HIRStatement) -> Result<(), String> {
    unsafe {
        let current_func = compiler.current_function.ok_or("[ERR-SEM-532] No active function")?;
        let while_cond_bb = LLVMAppendBasicBlockInContext(
            compiler.context,
            current_func,
            b"while_cond\0".as_ptr() as _,
        );
        let while_body_bb = LLVMAppendBasicBlockInContext(
            compiler.context,
            current_func,
            b"while_body\0".as_ptr() as _,
        );
        let while_exit_bb = LLVMAppendBasicBlockInContext(
            compiler.context,
            current_func,
            b"while_exit\0".as_ptr() as _,
        );

        LLVMBuildBr(compiler.builder, while_cond_bb);

        LLVMPositionBuilderAtEnd(compiler.builder, while_cond_bb);
        compiler.break_targets.push(while_exit_bb);

        if let HIRStatement::While { condition, body } = stmt {
            let (cond_val, ty) = compiler.codegen_expr(condition)?;
            if ty != HIRType::Bool {
                return Err("[ERR-SEM-534] while condition must be a boolean expression".into());
            }

            LLVMBuildCondBr(compiler.builder, cond_val, while_body_bb, while_exit_bb);

            LLVMPositionBuilderAtEnd(compiler.builder, while_body_bb);
            for s in body {
                super::codegen_statement(compiler, s)?;
                if !LLVMGetBasicBlockTerminator(LLVMGetInsertBlock(compiler.builder)).is_null() {
                    break;
                }
            }
            if LLVMGetBasicBlockTerminator(LLVMGetInsertBlock(compiler.builder)).is_null() {
                LLVMBuildBr(compiler.builder, while_cond_bb);
            }

            compiler.break_targets.pop();

            LLVMPositionBuilderAtEnd(compiler.builder, while_exit_bb);

            Ok(())
        } else {
            return Err("[ERR-SEM-533] codegen_while expected HIRStatement::While".into());
        }
    }
}