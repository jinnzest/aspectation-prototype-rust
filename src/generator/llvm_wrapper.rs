#![allow(dead_code)]
extern crate libc;
pub extern crate llvm_sys;

use self::llvm_sys::analysis::LLVMVerifierFailureAction;
use self::llvm_sys::analysis::LLVMVerifyModule;
use self::llvm_sys::core::*;
use self::llvm_sys::prelude::*;
use self::llvm_sys::target::*;
use self::llvm_sys::target_machine::*;
use semantic::model::*;
use std::ffi::CStr;
use std::ffi::CString;
use std::fs::remove_file;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

const DEFAULT_ADDRESS_SPACE: libc::c_uint = 0;

fn to_llvm_bool(b: bool) -> LLVMBool {
    if b {
        1
    } else {
        0
    }
}

macro_rules! empty_mut_c_str {
    ($s:expr) => {
        "\00".as_ptr() as *mut i8
    };
}

fn from_c(c_str: *const i8) -> String {
    let c_str: &CStr = unsafe { CStr::from_ptr(c_str) };
    let s = c_str.to_str().unwrap();
    s.to_owned()
}

struct CStrOwner {
    strings: Vec<CString>,
}

impl CStrOwner {
    fn new() -> Self {
        CStrOwner { strings: vec![] }
    }

    pub fn new_str_ptr(&mut self, s: &str) -> *mut i8 {
        let cstring = CString::new(s).unwrap();
        let ptr = cstring.as_ptr() as *mut _;
        self.strings.push(cstring);
        ptr
    }
}

#[derive(Clone, Copy)]
pub struct Types {
    pub i8t: LLVMTypeRef,
    pub i32t: LLVMTypeRef,
    pub i64t: LLVMTypeRef,
    pub void: LLVMTypeRef,
    pub mp_struct: LLVMTypeRef,
    pub file_struct: LLVMTypeRef,
}

pub struct LLVMWrapper {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    cstr_owner: CStrOwner,
    pub types: Types,
}

impl Default for LLVMWrapper {
    fn default() -> Self {
        println!("Initializing LLVM");
        unsafe {
            let context = LLVMContextCreate();
            let mut cstr_owner = CStrOwner::new();
            let module =
                LLVMModuleCreateWithNameInContext(cstr_owner.new_str_ptr("module"), context);
            let builder = LLVMCreateBuilderInContext(context);
            let types = Types {
                i8t: i8_t(context),
                i32t: i32_t(context),
                i64t: i64_t(context),
                void: void_t(context),
                mp_struct: mp_struct(context, cstr_owner.new_str_ptr("mp_struct")),
                file_struct: file_struct(context, &mut cstr_owner),
            };
            LLVMWrapper {
                context,
                module,
                builder,
                cstr_owner,
                types,
            }
        }
    }
}

impl LLVMWrapper {
    pub unsafe fn add_function(
        &mut self,
        name: &FuncName,
        function_ty: LLVMTypeRef,
    ) -> LLVMValueRef {
        LLVMAddFunction(
            self.module,
            self.cstr_owner.new_str_ptr(&name.str()),
            function_ty,
        )
    }

    pub unsafe fn build_alloca(&mut self, ty: LLVMTypeRef, name: &str) -> LLVMValueRef {
        LLVMBuildAlloca(self.builder, ty, self.cstr_owner.new_str_ptr(name))
    }

    pub unsafe fn build_load(&mut self, pointer_val: LLVMValueRef, name: &str) -> LLVMValueRef {
        LLVMBuildLoad(self.builder, pointer_val, self.cstr_owner.new_str_ptr(name))
    }

    pub unsafe fn build_store(&mut self, val: LLVMValueRef, ptr: LLVMValueRef) -> LLVMValueRef {
        LLVMBuildStore(self.builder, val, ptr)
    }

    pub unsafe fn build_call(
        &mut self,
        func: LLVMValueRef,
        args: &mut [LLVMValueRef],
        name: &str,
    ) -> LLVMValueRef {
        LLVMBuildCall(
            self.builder,
            func,
            args.as_mut_ptr(),
            args.len() as u32,
            self.cstr_owner.new_str_ptr(name),
        )
    }

    pub unsafe fn append_basic_block_in_context(
        &mut self,
        func: LLVMValueRef,
        name: &str,
    ) -> LLVMBasicBlockRef {
        LLVMAppendBasicBlockInContext(self.context, func, self.cstr_owner.new_str_ptr(name))
    }

    pub unsafe fn position_builder_at_end(&self, block: LLVMBasicBlockRef) {
        LLVMPositionBuilderAtEnd(self.builder, block)
    }

    pub unsafe fn build_global_string(&mut self, string: &str, name: &str) -> LLVMValueRef {
        LLVMBuildGlobalString(
            self.builder,
            self.cstr_owner.new_str_ptr(string),
            self.cstr_owner.new_str_ptr(name),
        )
    }

    pub unsafe fn build_s_ext(
        &mut self,
        val: LLVMValueRef,
        dest_ty: LLVMTypeRef,
        name: &str,
    ) -> LLVMValueRef {
        LLVMBuildSExt(
            self.builder,
            val,
            dest_ty,
            self.cstr_owner.new_str_ptr(name),
        )
    }

    pub unsafe fn build_trunc(
        &mut self,
        val: LLVMValueRef,
        dest_ty: LLVMTypeRef,
        name: &str,
    ) -> LLVMValueRef {
        LLVMBuildTrunc(
            self.builder,
            val,
            dest_ty,
            self.cstr_owner.new_str_ptr(name),
        )
    }

    pub unsafe fn build_bit_cast(
        &mut self,
        val: LLVMValueRef,
        dest_ty: LLVMTypeRef,
        name: &str,
    ) -> LLVMValueRef {
        LLVMBuildBitCast(
            self.builder,
            val,
            dest_ty,
            self.cstr_owner.new_str_ptr(name),
        )
    }

    pub unsafe fn build_struct_gep(
        &mut self,
        pointer: LLVMValueRef,
        idx: libc::c_uint,
        name: &str,
    ) -> LLVMValueRef {
        LLVMBuildStructGEP(
            self.builder,
            pointer,
            idx,
            self.cstr_owner.new_str_ptr(name),
        )
    }

    pub unsafe fn add_global(&mut self, ty: LLVMTypeRef, name: &str) -> LLVMValueRef {
        LLVMAddGlobal(self.module, ty, self.cstr_owner.new_str_ptr(name))
    }

    pub unsafe fn build_ret_void(&self) -> LLVMValueRef {
        LLVMBuildRetVoid(self.builder)
    }

    pub unsafe fn build_ret(&self, v: LLVMValueRef) -> LLVMValueRef {
        LLVMBuildRet(self.builder, v)
    }

    pub fn dump(&self, name: &str) {
        let file_name = format!("./target/{}.ll", name);
        println!("Dumping LLVM IR to the file: {}", file_name);
        if Path::new(&file_name).exists() {
            match remove_file(&file_name) {
                Err(e) => println!(
                    "The file '{}' can't be removed because of the error: {}",
                    file_name, e
                ),
                Ok(_) => {
                    writing_dump(&file_name, self.module);
                }
            }
        } else {
            writing_dump(&file_name, self.module);
        }
    }

    pub fn mk_object_file(&mut self, name: &str) -> bool {
        unsafe {
            println!("initializing LLVM to generate object file\n");
            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargets();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllAsmParsers();
            LLVM_InitializeAllAsmPrinters();

            let mut module_verification_error = empty_mut_c_str!("");
            LLVMVerifyModule(
                self.module,
                LLVMVerifierFailureAction::LLVMPrintMessageAction,
                &mut module_verification_error,
            );

            let triple = LLVMGetDefaultTargetTriple();
            println!("Triple: {:?}", from_c(triple));
            let cpu = LLVMGetHostCPUName();
            println!("CPU: {:?}", from_c(cpu));
            let features = LLVMGetHostCPUFeatures();
            //            println!("Features: {:?}", from_c(features));

            LLVMSetTarget(self.module, triple);

            let mut target = LLVMGetFirstTarget();
            println!("{:?}", from_c(LLVMGetTargetName(target)));

            let mut getting_target_error = empty_mut_c_str!("");

            if LLVMGetTargetFromTriple(triple, &mut target, &mut getting_target_error) == 1 {
                panic!("can't get target");
            }

            let getting_target_err_str = from_c(getting_target_error);

            if !getting_target_err_str.is_empty() {
                println!("Error getting target: {}", getting_target_err_str);
            }
            println!("creating target machine");
            let target_machine = LLVMCreateTargetMachine(
                target,
                triple,
                cpu,
                features,
                LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );

            LLVMCreateTargetDataLayout(target_machine);

            let file_name = format!("./target/{}.o\0", name).as_ptr() as *mut i8;
            println!("file name = {}", from_c(file_name));

            let mut error_emitting_obj = empty_mut_c_str!("");
            println!("creating object file");

            LLVMTargetMachineEmitToFile(
                target_machine,
                self.module,
                file_name,
                LLVMCodeGenFileType::LLVMObjectFile,
                &mut error_emitting_obj,
            );

            let emitting_obj_err_str = from_c(error_emitting_obj);

            LLVMDisposeTargetMachine(target_machine);

            if !emitting_obj_err_str.is_empty() {
                println!("ERROR generating file: {}", emitting_obj_err_str);
                false
            } else {
                true
            }
        }
    }
}

pub unsafe fn get_arg_operand(funclet: LLVMValueRef, i: libc::c_uint) -> LLVMValueRef {
    LLVMGetArgOperand(funclet, i)
}

pub unsafe fn get_param(func: LLVMValueRef, index: libc::c_uint) -> LLVMValueRef {
    LLVMGetParam(func, index)
}

pub unsafe fn ptr_t(element_type: LLVMTypeRef) -> LLVMTypeRef {
    LLVMPointerType(element_type, DEFAULT_ADDRESS_SPACE)
}

pub unsafe fn arr_t(t: LLVMTypeRef, cnt: libc::c_uint) -> LLVMTypeRef {
    LLVMArrayType(t, cnt)
}

unsafe fn set_alignment(v: LLVMValueRef, bytes: libc::c_uint) {
    LLVMSetAlignment(v, bytes);
}

pub unsafe fn set_param_alignment(arg: LLVMValueRef, align: libc::c_uint) {
    LLVMSetParamAlignment(arg, align);
}

pub unsafe fn struct_set_body(
    struct_ty: LLVMTypeRef,
    element_types: &mut [LLVMTypeRef],
    packed: bool,
) {
    LLVMStructSetBody(
        struct_ty,
        element_types.as_mut_ptr(),
        element_types.len() as u32,
        to_llvm_bool(packed),
    );
}

pub unsafe fn struct_type(element_types: &mut [LLVMTypeRef], packed: bool) -> LLVMTypeRef {
    LLVMStructType(
        element_types.as_mut_ptr(),
        element_types.len() as u32,
        to_llvm_bool(packed),
    )
}

pub unsafe fn function_type(
    return_type: LLVMTypeRef,
    param_types: &mut [LLVMTypeRef],
    is_var_arg: bool,
) -> LLVMTypeRef {
    LLVMFunctionType(
        return_type,
        param_types.as_mut_ptr(),
        param_types.len() as u32,
        to_llvm_bool(is_var_arg),
    )
}

pub unsafe fn const_int(
    int_ty: LLVMTypeRef,
    n: libc::c_ulonglong,
    sign_extend: LLVMBool,
) -> LLVMValueRef {
    LLVMConstInt(int_ty, n, sign_extend)
}

fn i32_t(c: LLVMContextRef) -> LLVMTypeRef {
    unsafe { LLVMInt32TypeInContext(c) }
}

fn i16_t(c: LLVMContextRef) -> LLVMTypeRef {
    unsafe { LLVMInt16TypeInContext(c) }
}

fn i64_t(c: LLVMContextRef) -> LLVMTypeRef {
    unsafe { LLVMInt64TypeInContext(c) }
}

fn i8_t(c: LLVMContextRef) -> LLVMTypeRef {
    unsafe { LLVMInt8TypeInContext(c) }
}

fn void_t(c: LLVMContextRef) -> LLVMTypeRef {
    unsafe { LLVMVoidTypeInContext(c) }
}

unsafe fn mp_struct(c: LLVMContextRef, name: *mut libc::c_char) -> LLVMTypeRef {
    LLVMStructCreateNamed(c, name)
}

unsafe fn file_struct(c: LLVMContextRef, cstr_owner: &mut CStrOwner) -> LLVMTypeRef {
    LLVMStructCreateNamed(c, cstr_owner.new_str_ptr("__sFILE"))
}

impl Drop for LLVMWrapper {
    fn drop(&mut self) {
        println!("shutting down LLVM...");
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}

fn writing_dump(file_name: &str, module: LLVMModuleRef) {
    unsafe {
        let llvm_ir_ptr = LLVMPrintModuleToString(module);
        let llvm_ir = CStr::from_ptr(llvm_ir_ptr as *const _);
        match File::create(&file_name) {
            Ok(mut f) => match f.write_all(llvm_ir.to_bytes()) {
                Ok(_) => {}
                Err(e) => println!(
                    "The file '{}' can't be written because of the error: {}",
                    file_name, e
                ),
            },
            Err(e) => println!(
                "The file '{}' can't be created because of the error: {}",
                file_name, e
            ),
        }
    }
}
