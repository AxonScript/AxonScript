//this is the core compiler state for codegen.
//handles context/module/builder, tracks vars & funcs,
//and converts hir expressions into llvm ir

use crate::high_level_ir::{HIRExpr, HIRType};
use llvm_sys::LLVMLinkage;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::collections::{HashMap, HashSet};
use std::ffi::CString;

pub struct Compiler {
    pub context: LLVMContextRef,
    pub module: LLVMModuleRef,
    pub builder: LLVMBuilderRef,
    pub mutable_vars: HashSet<String>,
    pub variables: HashMap<String, (LLVMValueRef, HIRType)>,
    pub functions: HashMap<String, (LLVMValueRef, LLVMTypeRef, HIRType)>,
    pub current_function: Option<LLVMValueRef>,
    pub string_counter: usize,
    pub break_targets: Vec<LLVMBasicBlockRef>,
}

impl Compiler {
    pub fn new(module_name: &str) -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module_name = CString::new(module_name).unwrap();
            let module = LLVMModuleCreateWithNameInContext(module_name.as_ptr(), context);
            let builder = LLVMCreateBuilderInContext(context);
            let mut functions = HashMap::new();
            let i32_type = LLVMInt32TypeInContext(context);
            let i8_ptr_type = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
            let printf_type =
                LLVMFunctionType(i32_type, [i8_ptr_type].as_ptr() as *mut LLVMTypeRef, 1, 1);
            let printf_func =
                LLVMAddFunction(module, b"printf\0".as_ptr() as *const _, printf_type);
            functions.insert(
                "printf".to_string(),
                (printf_func, printf_type, HIRType::I32),
            );
            Compiler {
                context,
                module,
                builder,
                mutable_vars: HashSet::new(),
                variables: HashMap::new(),
                functions,
                current_function: None,
                string_counter: 0,
                break_targets: Vec::new(),
            }
        }
    }

    pub fn current_function_return_type(&self) -> Option<HIRType> {
        let cur = self.current_function?;
        for (_, (func, _llvm_ty, ret_ty)) in &self.functions {
            if *func == cur {
                return Some(ret_ty.clone());
            }
        }
        None
    }

    pub fn hir_type_to_llvm_type(&self, ty: &HIRType) -> LLVMTypeRef {
        unsafe {
            match ty {
                HIRType::I32 => LLVMInt32TypeInContext(self.context),
                HIRType::I64 => LLVMInt64TypeInContext(self.context),
                HIRType::F32 => LLVMFloatTypeInContext(self.context),
                HIRType::F64 => LLVMDoubleTypeInContext(self.context),
                HIRType::Bool => LLVMInt1TypeInContext(self.context),
                HIRType::String => LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                HIRType::Void => LLVMVoidTypeInContext(self.context),
            }
        }
    }

    pub fn codegen_expr(&mut self, expr: &HIRExpr) -> Result<(LLVMValueRef, HIRType), String> {
        unsafe {
            match expr {
                HIRExpr::Int32(val) => Ok((
                    LLVMConstInt(self.hir_type_to_llvm_type(&HIRType::I32), *val as u64, 1),
                    HIRType::I32,
                )),
                HIRExpr::Int64(val) => Ok((
                    LLVMConstInt(self.hir_type_to_llvm_type(&HIRType::I64), *val as u64, 1),
                    HIRType::I64,
                )),
                HIRExpr::Float32(val) => Ok((
                    LLVMConstReal(self.hir_type_to_llvm_type(&HIRType::F32), *val as f64),
                    HIRType::F32,
                )),
                HIRExpr::Float64(val) => Ok((
                    LLVMConstReal(self.hir_type_to_llvm_type(&HIRType::F64), *val),
                    HIRType::F64,
                )),
                HIRExpr::Bool(val) => Ok((
                    LLVMConstInt(self.hir_type_to_llvm_type(&HIRType::Bool), *val as u64, 0),
                    HIRType::Bool,
                )),
                HIRExpr::String(val) => {
                    let c_string = CString::new(val.as_str()).unwrap();
                    let str_name = CString::new(format!(".str{}", self.string_counter)).unwrap();
                    self.string_counter += 1;
                    let const_str_val =
                        LLVMConstStringInContext(self.context, c_string.as_ptr(), val.len() as u32, 0);
                    let global =
                        LLVMAddGlobal(self.module, LLVMTypeOf(const_str_val), str_name.as_ptr());
                    LLVMSetLinkage(global, LLVMLinkage::LLVMPrivateLinkage);
                    LLVMSetInitializer(global, const_str_val);
                    LLVMSetGlobalConstant(global, 1);
                    let zero_indices = [
                        LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0),
                        LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0),
                    ];
                    let gep = LLVMConstInBoundsGEP2(
                        LLVMTypeOf(const_str_val),
                        global,
                        zero_indices.as_ptr() as *mut LLVMValueRef,
                        2,
                    );
                    Ok((gep, HIRType::String))
                }
                HIRExpr::Identifier(name) => {
                    let var_data = self
                        .variables
                        .get(name)
                        .ok_or_else(|| format!("Unknown variable: {}", name))?;
                    let (ptr, ty) = var_data;
                    let var_type_ref = self.hir_type_to_llvm_type(ty);
                    let name_c = CString::new(name.as_str()).unwrap();
                    let loaded_val =
                        LLVMBuildLoad2(self.builder, var_type_ref, *ptr, name_c.as_ptr());
                    Ok((loaded_val, ty.clone()))
                }
                HIRExpr::BinaryOp { left, op, right } => {
                    super::compiler_math_codegen::codegen_math_expr(self, left, op, right, None)
                }
                HIRExpr::FunctionCall { name, args } => {
                    let (func, func_type, return_hir_type) = self
                        .functions
                        .get(name)
                        .cloned()
                        .ok_or_else(|| format!("Unknown function: {}", name))?;
                    let mut arg_values = Vec::with_capacity(args.len());
                    for arg_expr in args {
                        let (arg_val, _) = self.codegen_expr(arg_expr)?;
                        arg_values.push(arg_val);
                    }
                    let call = LLVMBuildCall2(
                        self.builder,
                        func_type,
                        func,
                        arg_values.as_mut_ptr(),
                        arg_values.len() as u32,
                        b"call\0".as_ptr() as *const _,
                    );
                    Ok((call, return_hir_type))
                }
                HIRExpr::Coerce { expr, target } => {
                    let (val, from_ty) = self.codegen_expr(expr)?;
                    let to_llvm_ty = self.hir_type_to_llvm_type(target);
                    let casted = match (&from_ty, target) {
                        (HIRType::I32, HIRType::I64) => {
                            LLVMBuildSExt(self.builder, val, to_llvm_ty, b"sext\0".as_ptr() as _)
                        }
                        (HIRType::I32, HIRType::F64) => LLVMBuildSIToFP(
                            self.builder,
                            val,
                            to_llvm_ty,
                            b"sitofp\0".as_ptr() as _,
                        ),
                        (HIRType::I64, HIRType::F64) => LLVMBuildSIToFP(
                            self.builder,
                            val,
                            to_llvm_ty,
                            b"sitofp\0".as_ptr() as _,
                        ),
                        (HIRType::F32, HIRType::F64) => {
                            LLVMBuildFPExt(self.builder, val, to_llvm_ty, b"fpext\0".as_ptr() as _)
                        }
                        _ => {
                            return Err(format!(
                                "[ERR-SEM-511] Unsupported coercion from {:?} to {:?}",
                                from_ty, target
                            ));
                        }
                    };
                    Ok((casted, target.clone()))
                }
            }
        }
    }

    pub fn dispose(self) {
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}