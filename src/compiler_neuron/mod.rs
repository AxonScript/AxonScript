//compiler kernel, 
//all modules are designated here, 
//and also here is the main logic of converting HIR to llvm ir


use crate::high_level_ir::{HIRStatement, HIRType};
use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm_sys::core::*;
use llvm_sys::execution_engine::{
    LLVMCreateExecutionEngineForModule, LLVMDisposeExecutionEngine, LLVMExecutionEngineRef,
    LLVMGetFunctionAddress,
};
use llvm_sys::prelude::LLVMModuleRef;
use llvm_sys::target::{
    LLVM_InitializeAllAsmPrinters, LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs,
    LLVM_InitializeAllTargets, LLVM_InitializeNativeAsmParser, LLVM_InitializeNativeAsmPrinter,
    LLVM_InitializeNativeTarget,
};
use llvm_sys::target_machine::{
    LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetMachine,
    LLVMDisposeTargetMachine, LLVMGetTargetFromTriple, LLVMRelocMode,
    LLVMTargetMachineEmitToFile,
};
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::fmt;

pub mod compiler_context;
pub mod compiler_function_codegen;
pub mod compiler_if_codegen;
pub mod compiler_input_codegen;
pub mod compiler_loop_codegen;
pub mod compiler_math_codegen;
pub mod compiler_print_codegen;
pub mod compiler_variable_codegen;

#[derive(Debug)]
pub struct CompilerError(pub String);

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Compiler Error: {}", self.0)
    }
}

impl From<String> for CompilerError {
    fn from(error: String) -> Self {
        CompilerError(error)
    }
}

pub type CompileResult<T> = Result<T, Vec<CompilerError>>;

pub fn codegen_statement(
    compiler: &mut compiler_context::Compiler,
    stmt: &HIRStatement,
) -> Result<(), String> {
    match stmt {
        HIRStatement::Function { .. } => {
            compiler_function_codegen::codegen_function(compiler, stmt)
        }
        HIRStatement::Assignment { .. } => {
            compiler_variable_codegen::codegen_assignment(compiler, stmt)
        }
        HIRStatement::Print { .. } => compiler_print_codegen::codegen_print(compiler, stmt),
        HIRStatement::ExprStatement { expr } => compiler.codegen_expr(expr).map(|_| ()),
        HIRStatement::If { .. } => compiler_if_codegen::codegen_if(compiler, stmt),
        HIRStatement::While { .. } => compiler_loop_codegen::codegen_while(compiler, stmt),
        HIRStatement::Loop { .. } => compiler_loop_codegen::codegen_loop(compiler, stmt),
        HIRStatement::Break => compiler_loop_codegen::codegen_break(compiler),
        HIRStatement::Input { .. } => compiler_input_codegen::codegen_input(compiler, stmt),
    }
}

pub fn create_llvm_module(
    hir: Vec<HIRStatement>,
    mutable_vars: HashSet<String>,
) -> Result<compiler_context::Compiler, Vec<CompilerError>> {
    let mut compiler = compiler_context::Compiler::new("axon_module");
    compiler.mutable_vars = mutable_vars;
    let mut errors: Vec<CompilerError> = Vec::new();

    unsafe {
        for statement in &hir {
            if let HIRStatement::Function { name, params, return_type, start, .. } = statement {
                let final_name = if *start { "main" } else { name };
                let func_name = CString::new(final_name).unwrap();
                let param_types_llvm: Vec<_> = params
                    .iter()
                    .map(|(_, ty)| compiler.hir_type_to_llvm_type(ty))
                    .collect();
                let ret_type_llvm = if *start {
                    LLVMInt32TypeInContext(compiler.context)
                } else {
                    compiler.hir_type_to_llvm_type(return_type)
                };
                let func_type = LLVMFunctionType(
                    ret_type_llvm,
                    param_types_llvm.as_ptr() as *mut _,
                    param_types_llvm.len() as u32,
                    0,
                );
                let llvm_func = LLVMAddFunction(compiler.module, func_name.as_ptr(), func_type);
                let recorded_ret_type = if *start { HIRType::I32 } else { return_type.clone() };
                compiler.functions.insert(name.clone(), (llvm_func, func_type, recorded_ret_type));
            }
        }

        for statement in hir {
            if let Err(e) = codegen_statement(&mut compiler, &statement) {
                errors.push(CompilerError(e));
            }
        }
    }

    if !errors.is_empty() {
        compiler.dispose();
        return Err(errors);
    }

    Ok(compiler)
}

pub fn compile_and_run_jit(
    hir: Vec<HIRStatement>,
    mutable_vars: HashSet<String>,
) -> CompileResult<()> {
    unsafe {
        LLVM_InitializeNativeTarget();
        LLVM_InitializeNativeAsmPrinter();
        LLVM_InitializeNativeAsmParser();

        let compiler = create_llvm_module(hir, mutable_vars)?;
        let module = compiler.module;

        let mut error_msg: *mut i8 = std::ptr::null_mut();
        if LLVMVerifyModule(module, LLVMVerifierFailureAction::LLVMReturnStatusAction, &mut error_msg) == 1 {
            let msg = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
            LLVMDisposeMessage(error_msg);
            compiler.dispose();
            return Err(vec![CompilerError(format!("LLVM module verification failed: {}", msg))]);
        }
        
        let mut ee: LLVMExecutionEngineRef = std::ptr::null_mut();
        if LLVMCreateExecutionEngineForModule(&mut ee, module, &mut error_msg) != 0 {
            let msg = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
            LLVMDisposeMessage(error_msg);
            // Do not dispose compiler here, as EE creation failed before it could take ownership
            compiler.dispose();
            return Err(vec![CompilerError(format!("Failed to create JIT execution engine: {}", msg))]);
        }

        let main_func_name = CString::new("main").unwrap();
        let main_func_addr = LLVMGetFunctionAddress(ee, main_func_name.as_ptr());

        let exit_code = if main_func_addr == 0 {
            -1 
        } else {
            type MainFn = unsafe extern "C" fn() -> i32;
            let main_fn = std::mem::transmute::<u64, MainFn>(main_func_addr);
            main_fn()
        };
        
        LLVMDisposeExecutionEngine(ee);

        // Manually dispose of non-module resources, as the EE now owns the module.
        LLVMDisposeBuilder(compiler.builder);
        LLVMContextDispose(compiler.context);
        
        if exit_code != 0 {
             if main_func_addr == 0 {
                return Err(vec![CompilerError("Main function not found".to_string())]);
            }
            return Err(vec![CompilerError(format!("Program exited with code: {}", exit_code))]);
        }
    }
    Ok(())
}

pub fn emit_object_file(
    module: LLVMModuleRef,
    target_triple_str: &str,
    output_filename: &str,
) -> Result<(), String> {
    unsafe {
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmPrinters();

        let mut error_msg: *mut i8 = std::ptr::null_mut();
        if LLVMVerifyModule(module, LLVMVerifierFailureAction::LLVMReturnStatusAction, &mut error_msg) == 1 {
            let msg = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
            LLVMDisposeMessage(error_msg);
            return Err(format!("LLVM module verification failed: {}", msg));
        }

        let target_triple = CString::new(target_triple_str).unwrap();
        let mut target = std::ptr::null_mut();

        if LLVMGetTargetFromTriple(target_triple.as_ptr(), &mut target, &mut error_msg) != 0 {
            let msg = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
            LLVMDisposeMessage(error_msg);
            return Err(format!("Failed to get target from triple: {}", msg));
        }

        let cpu = CString::new("generic").unwrap();
        let features = CString::new("").unwrap();
        let target_machine = LLVMCreateTargetMachine(
            target,
            target_triple.as_ptr(),
            cpu.as_ptr(),
            features.as_ptr(),
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            LLVMRelocMode::LLVMRelocPIC, // <-- FIX for PIE
            LLVMCodeModel::LLVMCodeModelDefault,
        );

        if target_machine.is_null() {
            return Err("Failed to create target machine".to_string());
        }

        LLVMSetTarget(module, target_triple.as_ptr());

        let filename_c = CString::new(output_filename).unwrap();
        if LLVMTargetMachineEmitToFile(
            target_machine,
            module,
            filename_c.as_ptr() as *mut i8,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error_msg,
        ) != 0 {
            let msg = CStr::from_ptr(error_msg).to_string_lossy().into_owned();
            LLVMDisposeMessage(error_msg);
            LLVMDisposeTargetMachine(target_machine);
            return Err(format!("Failed to emit object file: {}", msg));
        }

        LLVMDisposeTargetMachine(target_machine);
    }
    Ok(())
}