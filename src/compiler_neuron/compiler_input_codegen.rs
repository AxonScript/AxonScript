//generating llvm ir to take text from terminal, 
//here I used scanf method

use super::compiler_context::Compiler;
use crate::high_level_ir::{HIRExpr, HIRStatement, HIRType};
use llvm_sys::core::*;
use std::ffi::CString;

pub fn codegen_input(compiler: &mut Compiler, stmt: &HIRStatement) -> Result<(), String> {
    unsafe {
        if let HIRStatement::Input { target } = stmt {
            let HIRExpr::Identifier(name) = target else {
                return Err("\x1b[31m[ERR-SEM-542] input target must be identifier\x1b[0m".into());
            };
            if !compiler.mutable_vars.contains(name) {
                return Err(format!(
                    "\x1b[31m[ERR-SEM-549] cannot input into immutable variable '{}'\x1b[0m",
                    name
                ));
            }
            let (var_ptr, var_ty) = compiler
                .variables
                .get(name)
                .ok_or("\x1b[31m[ERR-SEM-543] variable not declared for input\x1b[0m")?;
            if LLVMIsAGlobalVariable(*var_ptr) != std::ptr::null_mut() {
                return Err(format!(
                    "\x1b[31m[ERR-SEM-548] input on global variable '{}' is UB: use only local (alloca) variables!\x1b[0m",
                    name
                ));
            }
            if (*var_ptr).is_null() {
                return Err(format!(
                    "\x1b[31m[ERR-SEM-690] null ptr for '{}'\x1b[0m",
                    name
                ));
            }

            let i8_type = LLVMInt8TypeInContext(compiler.context);
            let i8_ptr_type = LLVMPointerType(i8_type, 0);

            let scanf_name = CString::new("scanf").unwrap();
            let scanf_ty = LLVMFunctionType(
                LLVMInt32TypeInContext(compiler.context),
                [i8_ptr_type].as_ptr() as *mut _,
                1,
                1,
            );
            let scanf_func = {
                let f = LLVMGetNamedFunction(compiler.module, scanf_name.as_ptr());
                if f.is_null() {
                    LLVMAddFunction(compiler.module, scanf_name.as_ptr(), scanf_ty)
                } else {
                    f
                }
            };

            let fmt_str = match var_ty {
                HIRType::I32 | HIRType::Bool => "%d",
                HIRType::I64 => "%lld",
                HIRType::F32 => "%f",
                HIRType::F64 => "%lf",
                HIRType::String => "%s",
                _ => return Err("\x1b[31m[ERR-SEM-544] unsupported input type\x1b[0m".into()),
            };
            let fmt_c = CString::new(fmt_str).unwrap();
            let fmt_ptr = LLVMBuildGlobalStringPtr(
                compiler.builder,
                fmt_c.as_ptr(),
                b"scanf_fmt\0".as_ptr() as _,
            );

            if *var_ty == HIRType::String {
                let malloc_name = CString::new("malloc").unwrap();
                let malloc_ty = LLVMFunctionType(
                    i8_ptr_type,
                    [LLVMInt64TypeInContext(compiler.context)].as_ptr() as *mut _,
                    1,
                    0,
                );
                let malloc_func = {
                    let f = LLVMGetNamedFunction(compiler.module, malloc_name.as_ptr());
                    if f.is_null() {
                        LLVMAddFunction(compiler.module, malloc_name.as_ptr(), malloc_ty)
                    } else {
                        f
                    }
                };
                let size_val = LLVMConstInt(LLVMInt64TypeInContext(compiler.context), 1024, 0);
                let mut malloc_args = [size_val];
                let buf_ptr = LLVMBuildCall2(
                    compiler.builder,
                    malloc_ty,
                    malloc_func,
                    malloc_args.as_mut_ptr(),
                    1,
                    b"malloc_call\0".as_ptr() as _,
                );
                if buf_ptr.is_null() {
                    return Err(format!(
                        "\x1b[31m[ERR-SEM-691] malloc returned null for '{}'\x1b[0m",
                        name
                    ));
                }
                let mut args = [fmt_ptr, buf_ptr];
                let res = LLVMBuildCall2(
                    compiler.builder,
                    scanf_ty,
                    scanf_func,
                    args.as_mut_ptr(),
                    2,
                    b"scanf_call\0".as_ptr() as _,
                );
                if res.is_null() {
                    return Err(format!(
                        "\x1b[31m[ERR-SEM-692] scanf returned null for '{}'\x1b[0m",
                        name
                    ));
                }
                LLVMBuildStore(compiler.builder, buf_ptr, *var_ptr);
            } else {
                if *var_ty == HIRType::Bool {
                    let tmp_ptr = LLVMBuildAlloca(
                        compiler.builder,
                        LLVMInt32TypeInContext(compiler.context),
                        b"bool_tmp\0".as_ptr() as _,
                    );
                    let mut args = [fmt_ptr, tmp_ptr];
                    let res = LLVMBuildCall2(
                        compiler.builder,
                        scanf_ty,
                        scanf_func,
                        args.as_mut_ptr(),
                        2,
                        b"scanf_call\0".as_ptr() as _,
                    );
                    if res.is_null() {
                        return Err(format!(
                            "\x1b[31m[ERR-SEM-693] scanf returned null for '{}'\x1b[0m",
                            name
                        ));
                    }
                    let loaded = LLVMBuildLoad2(
                        compiler.builder,
                        LLVMInt32TypeInContext(compiler.context),
                        tmp_ptr,
                        b"bool_tmp_load\0".as_ptr() as _,
                    );
                    let truncated = LLVMBuildTrunc(
                        compiler.builder,
                        loaded,
                        LLVMInt1TypeInContext(compiler.context),
                        b"bool_trunc\0".as_ptr() as _,
                    );
                    LLVMBuildStore(compiler.builder, truncated, *var_ptr);
                } else {
                    let mut args = [fmt_ptr, *var_ptr];
                    let res = LLVMBuildCall2(
                        compiler.builder,
                        scanf_ty,
                        scanf_func,
                        args.as_mut_ptr(),
                        2,
                        b"scanf_call\0".as_ptr() as _,
                    );
                    if res.is_null() {
                        return Err(format!(
                            "\x1b[31m[ERR-SEM-693] scanf returned null for '{}'\x1b[0m",
                            name
                        ));
                    }
                }
            }
            Ok(())
        } else {
            Err("\x1b[31m[ERR-SEM-541] codegen_input expected HIRStatement::Input\x1b[0m".into())
        }
    }
}
