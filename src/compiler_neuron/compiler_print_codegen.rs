//generating llvm ir to output data to the terminal, 
//to implement this I used the printf method

use super::compiler_context::Compiler;
use crate::high_level_ir::{HIRStatement, HIRType};
use llvm_sys::core::*;
use llvm_sys::prelude::LLVMValueRef;
use std::ffi::CString;

unsafe fn get_format_string(
    compiler: &mut Compiler,
    ty: &HIRType,
    with_newline: bool,
) -> Result<LLVMValueRef, String> { unsafe {
    let format_str = match ty {
        HIRType::I32 => {
            if with_newline {
                "%d\n"
            } else {
                "%d"
            }
        }
        HIRType::I64 => {
            if with_newline {
                "%lld\n"
            } else {
                "%lld"
            }
        }
        HIRType::F32 | HIRType::F64 => {
            if with_newline {
                "%f\n"
            } else {
                "%f"
            }
        }
        HIRType::String => {
            if with_newline {
                "%s\n"
            } else {
                "%s"
            }
        }
        HIRType::Bool => {
            if with_newline {
                "%d\n"
            } else {
                "%d"
            }
        }
        _ => {
            return Err(format!(
                "[ERR-SEM-540] Unsupported type for printing: {:?}",
                ty
            ))
        }
    };
    let fmt_name = format!(
        ".fmt_{}{}",
        format_str.trim_end_matches(['\n']),
        compiler.string_counter
    );
    compiler.string_counter += 1;
    let c_format_str = CString::new(format_str).unwrap();
    let fmt_name_c = CString::new(fmt_name).unwrap();
    Ok(LLVMBuildGlobalStringPtr(
        compiler.builder,
        c_format_str.as_ptr(),
        fmt_name_c.as_ptr(),
    ))
}}

pub fn codegen_print(compiler: &mut Compiler, stmt: &HIRStatement) -> Result<(), String> {
    unsafe {
        if let HIRStatement::Print { params } = stmt {
            let (printf_func, printf_type, _) = compiler.functions.get("printf").cloned().unwrap();

            for (i, expr) in params.iter().enumerate() {
                let (mut value, ty) = compiler.codegen_expr(expr)?;

                if ty == HIRType::F32 {
                    value = LLVMBuildFPExt(
                        compiler.builder,
                        value,
                        compiler.hir_type_to_llvm_type(&HIRType::F64),
                        b"fpext\0".as_ptr() as *const _,
                    );
                }

                let is_last = i == params.len() - 1;
                let format_string = get_format_string(compiler, &ty, is_last)?;

                let mut args = vec![format_string, value];

                LLVMBuildCall2(
                    compiler.builder,
                    printf_type,
                    printf_func,
                    args.as_mut_ptr(),
                    args.len() as u32,
                    b"printcall\0".as_ptr() as *const _,
                );

                if !is_last {
                    let fmt_space = get_format_string(compiler, &HIRType::String, false)?;
                    let space_cstr = CString::new(" ").unwrap();
                    let space_val = LLVMBuildGlobalStringPtr(
                        compiler.builder,
                        space_cstr.as_ptr(),
                        b".sp\0".as_ptr() as *const _,
                    );
                    let mut space_args = vec![fmt_space, space_val];
                    LLVMBuildCall2(
                        compiler.builder,
                        printf_type,
                        printf_func,
                        space_args.as_mut_ptr(),
                        space_args.len() as u32,
                        b"printspace\0".as_ptr() as *const _,
                    );
                }
            }
            Ok(())
        } else {
            Err("[ERR-SEM-545] Provided statement is not a print statement".to_string())
        }
    }
}
