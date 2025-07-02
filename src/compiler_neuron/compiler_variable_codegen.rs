
//llvm ir generation for creating variables

use super::compiler_context::Compiler;
use crate::high_level_ir::{HIRStatement, HIRType};
use llvm_sys::core::*;
use std::ffi::CString;

pub fn codegen_assignment(compiler: &mut Compiler, stmt: &HIRStatement) -> Result<(), String> {
    unsafe {
        if let HIRStatement::Assignment { name, value } = stmt {
            let (val_ref, val_type) = compiler.codegen_expr(value)?;
            if val_ref.is_null() {
                return Err("\x1b[31m[ERR-SEM-695] Null value reference\x1b[0m".to_string());
            }
            let var_type_ref = compiler.hir_type_to_llvm_type(&val_type);
            let var_name_c = CString::new(name.as_str()).unwrap();

            let (ptr_to_store_to, is_global) = if let Some((existing_ptr, existing_type)) = compiler.variables.get(name) {
            
                if *existing_type != val_type {
                    return Err(format!(
                        "\x1b[31m[ERR-SEM-510] Type mismatch: existing var {:?} vs new value {:?}\x1b[0m",
                        existing_type, val_type
                    ));
                }
                (*existing_ptr, true) 
            } else if compiler.current_function.is_some() {
            
                let current_block = LLVMGetInsertBlock(compiler.builder);
                if current_block.is_null() {
                    return Err("\x1b[31m[ERR-SEM-670] Cannot allocate variable outside of a basic block\x1b[0m".to_string());
                }
                let alloca = LLVMBuildAlloca(compiler.builder, var_type_ref, var_name_c.as_ptr());
                if alloca.is_null() {
                    return Err("\x1b[31m[ERR-SEM-671] Failed to allocate local variable\x1b[0m".to_string());
                }
                compiler.variables.insert(name.clone(), (alloca, val_type.clone()));
                (alloca, false)
            } else {
        
                let global = LLVMAddGlobal(compiler.module, var_type_ref, var_name_c.as_ptr());
                if global.is_null() {
                    return Err("\x1b[31m[ERR-SEM-680] Failed to create global variable\x1b[0m".to_string());
                }
                LLVMSetInitializer(global, LLVMConstNull(var_type_ref));
                LLVMSetLinkage(global, llvm_sys::LLVMLinkage::LLVMExternalLinkage);
                compiler.variables.insert(name.clone(), (global, val_type.clone()));
                (global, true)
            };

            if is_global && compiler.current_function.is_none() {
                LLVMSetInitializer(ptr_to_store_to, val_ref);
            } else {
                let store = LLVMBuildStore(compiler.builder, val_ref, ptr_to_store_to);
                if store.is_null() {
                    return Err("\x1b[31m[ERR-SEM-694] Failed to store value\x1b[0m".to_string());
                }
            }
            Ok(())
        } else {
            Err("\x1b[31m[ERR-SEM-541] Provided statement is not an assignment\x1b[0m".to_string())
        }
    }
}