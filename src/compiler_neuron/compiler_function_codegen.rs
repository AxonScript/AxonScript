use super::compiler_context::Compiler;
use crate::high_level_ir::{HIRStatement, HIRType};
use llvm_sys::core::*;
use std::ffi::CString;

pub fn codegen_function(compiler: &mut Compiler, func: &HIRStatement) -> Result<(), String> {
    unsafe {
        if let HIRStatement::Function {
            name,
            params,
            start,
            body,
            return_type,
        } = func
        {
            let (llvm_func, _, _) = compiler
                .functions
                .get(name)
                .ok_or_else(|| format!("Function {} not pre-declared", name))?;

            let llvm_func_ref = *llvm_func;

            let entry_block = LLVMAppendBasicBlockInContext(
                compiler.context,
                llvm_func_ref,
                b"entry\0".as_ptr() as *const _,
            );
            LLVMPositionBuilderAtEnd(compiler.builder, entry_block);

            let old_vars = compiler.variables.clone();
            compiler.current_function = Some(llvm_func_ref);

            for (i, (param_name, param_ty)) in params.iter().enumerate() {
                let param_val = LLVMGetParam(llvm_func_ref, i as u32);
                let param_name_c = CString::new(format!("param_{}_{}", param_name, i)).unwrap();
                LLVMSetValueName2(
                    param_val,
                    param_name_c.as_ptr(),
                    param_name_c.as_bytes().len(),
                );

                let alloca = LLVMBuildAlloca(
                    compiler.builder,
                    compiler.hir_type_to_llvm_type(param_ty),
                    param_name_c.as_ptr(),
                );
                LLVMBuildStore(compiler.builder, param_val, alloca);
                compiler
                    .variables
                    .insert(param_name.clone(), (alloca, param_ty.clone()));
            }

            for statement in body {
                super::codegen_statement(compiler, statement)?;
            }

            compiler.variables = old_vars;
            compiler.current_function = None;

            let mut block = LLVMGetFirstBasicBlock(llvm_func_ref);
            while !block.is_null() {
                if LLVMGetBasicBlockTerminator(block).is_null() {
                    LLVMPositionBuilderAtEnd(compiler.builder, block);
                    if *start {
                        LLVMBuildRet(
                            compiler.builder,
                            LLVMConstInt(LLVMInt32TypeInContext(compiler.context), 0, 0),
                        );
                    } else if *return_type == HIRType::Void {
                        LLVMBuildRetVoid(compiler.builder);
                    } else {
                        return Err(format!(
                            "[ERR-SEM-520] Function '{}' with return type {:?} is missing a return statement at the end of its body.",
                            name, return_type
                        ));
                    }
                }
                block = LLVMGetNextBasicBlock(block);
            }

            Ok(())
        } else {
            Err("[ERR-SEM-523] Provided statement is not a function".to_string())
        }
    }
}