use std::ffi::{CString, CStr};

mod ffi {

    #![allow(improper_ctypes)]

    use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_ulonglong};
    use std::ffi::{CString, CStr};
    use std::marker::PhantomData;

    mod _repr {
        #[derive(Debug)]
        pub(super) struct LLVMBuilderRef;
        #[derive(Debug)]
        pub(super) struct LLVMContextRef;
        #[derive(Debug)]
        pub(super) struct LLVMBasicBlockRef;
        #[derive(Debug)]
        pub(super) struct LLVMModuleRef;
        #[derive(Debug)]
        pub(super) struct LLVMTypeRef;
        #[derive(Debug)]
        pub(super) struct LLVMValueRef;
    }

    type Ctx<'a> = PhantomData<fn() -> &'a ()>;
    type LLVMBool = c_int;
    type LLVMBuilderRef = *mut _repr::LLVMBuilderRef;
    type LLVMContextRef = *mut _repr::LLVMContextRef;
    type LLVMBasicBlockRef = *mut _repr::LLVMBasicBlockRef;
    type LLVMModuleRef = *mut _repr::LLVMModuleRef;
    type LLVMTypeRef = *mut _repr::LLVMTypeRef;
    type LLVMValueRef = *mut _repr::LLVMValueRef;

    extern "C" {
        fn LLVMContextCreate() -> LLVMContextRef;
        fn LLVMContextDispose(Context: LLVMContextRef);
    }

    pub struct Context(LLVMContextRef);

    impl Context {
        pub fn new() -> Self {
            Self(unsafe { LLVMContextCreate() })
        }
    }

    impl Drop for Context {
        fn drop(&mut self) {
            unsafe {
                LLVMContextDispose(self.0)
            }
        }
    }

    extern "C" {
        fn LLVMModuleCreateWithNameInContext(ModuleID: *const c_char, C: LLVMContextRef) -> LLVMModuleRef;
        fn LLVMDisposeModule(M: LLVMModuleRef);
        fn LLVMAddFunction(M: LLVMModuleRef, Name: *const c_char, FunctionTy: LLVMTypeRef) -> LLVMValueRef;
        fn LLVMPrintModuleToString(M: LLVMModuleRef) -> *const c_char;
    }

    pub struct Module<'ctx>(LLVMModuleRef, Ctx<'ctx>);
    impl<'ctx> Module<'ctx> {
        pub fn new(module_id: impl AsRef<str>, context: &'ctx Context) -> Self {
            let module_id = CString::new(module_id.as_ref()).unwrap();
            Module(unsafe {
                LLVMModuleCreateWithNameInContext(module_id.as_ptr(), context.0)
            }, PhantomData)
        }

        pub fn add_function(&self, name: &str, ty: FnType) -> FnValue<'ctx> {
            let name = CString::new(name).unwrap();
            FnValue(unsafe {
                LLVMAddFunction(self.0, name.as_ptr(), ty.0)
            }, PhantomData)
        }
    }

    impl Drop for Module<'_> {
        fn drop(&mut self) {
            unsafe {
                LLVMDisposeModule(self.0)
            }
        }
    }

    impl<'ctx> std::fmt::Display for Module<'ctx> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let content = unsafe {
                CStr::from_ptr(LLVMPrintModuleToString(self.0))
            }.to_str().unwrap();
            write!(f, "{content}")
        }
    }

    extern "C" {
        fn LLVMInt64TypeInContext(C: LLVMContextRef) -> LLVMTypeRef;
        fn LLVMGetIntTypeWidth(IntegerTy: LLVMTypeRef) -> c_uint;
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct Type<'ctx>(LLVMTypeRef, Ctx<'ctx>);
    impl<'ctx> From<IntType<'ctx>> for Type<'ctx> {
        fn from(ty: IntType<'ctx>) -> Self {
            Self(ty.0, PhantomData)
        }
    }
    impl<'ctx> From<FnType<'ctx>> for Type<'ctx> {
        fn from(ty: FnType) -> Self {
            Self(ty.0, PhantomData)
        }
    }

    #[derive(Copy, Clone)]
    pub struct IntType<'ctx>(LLVMTypeRef, Ctx<'ctx>);
    impl<'ctx> IntType<'ctx> {
        pub fn int64(context: &'ctx Context) -> Self {
            Self(unsafe {
                LLVMInt64TypeInContext(context.0)
            }, PhantomData)
        }

        pub fn width(&self) -> u32 {
            unsafe {
                LLVMGetIntTypeWidth(self.0).into()
            }
        }
    }
    impl std::fmt::Debug for IntType<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.debug_struct("IntType")
                .field("width", &self.width())
                .finish()
        }
    }

    extern "C" {
        fn LLVMFunctionType(ReturnType: LLVMTypeRef, ParamTypes: *mut Type, ParamCount: c_uint,
            IsVarArg: LLVMBool) -> LLVMTypeRef;
        fn LLVMIsFunctionVarArg(FunctionTy: LLVMTypeRef) -> LLVMBool;
        fn LLVMGetReturnType(FunctionTy: LLVMTypeRef) -> LLVMTypeRef;
        fn LLVMCountParamTypes(FunctionTy: LLVMTypeRef) -> c_uint;
        fn LLVMGetParamTypes(FunctionTy: LLVMTypeRef, Dest: *mut Type); 
        fn LLVMCountBasicBlocks(r#Fn: LLVMValueRef) -> c_uint;
    }

    #[derive(Copy, Clone)]
    pub struct FnType<'ctx>(LLVMTypeRef, Ctx<'ctx>);
    impl<'ctx> FnType<'ctx> {
        pub fn new(return_ty: impl Into<Type<'ctx>>, params: &mut [Type<'ctx>], is_var_arg: bool) -> Self {
            Self(unsafe {
                let return_ty = return_ty.into();
                let count = params.len() as c_uint;
                let is_var_arg = is_var_arg as LLVMBool;
                LLVMFunctionType(return_ty.0, params.as_mut_ptr(), count, is_var_arg)
            }, PhantomData)
        }

        pub fn is_var_arg(&self) -> bool {
            unsafe {
                LLVMIsFunctionVarArg(self.0) != 0
            }
        }

        pub fn return_type(&self) -> Type<'ctx> {
            Type(unsafe {
                LLVMGetReturnType(self.0)
            }, PhantomData )
        }

        pub fn param_types(&self) -> Vec<Type<'ctx>> {
            let count = unsafe { LLVMCountParamTypes(self.0) } as usize;
            let mut out = Vec::with_capacity(count);
            unsafe {
                LLVMGetParamTypes(self.0, out.as_mut_ptr())
            }
            out
        }
    }

    extern "C" {
        fn LLVMTypeOf(Val: LLVMValueRef) -> LLVMTypeRef;
        fn LLVMGetValueName2(Val: LLVMValueRef, Length: *mut c_ulong) -> *const c_char;
        fn LLVMSetValueName2(Val: LLVMValueRef, Name: *const c_char, Length: c_ulong);
        fn LLVMIsConstant(Val: LLVMValueRef) -> LLVMBool;
    }

    #[derive(Copy, Clone)]
    pub struct Value<'ctx>(LLVMValueRef, Ctx<'ctx>);
    impl<'ctx> From<FnValue<'ctx>> for Value<'ctx> {
        fn from(value: FnValue<'ctx>) -> Value<'ctx> {
            Value(value.0, PhantomData)
        }
    }
    impl<'ctx> From<IntValue<'ctx>> for Value<'ctx> {
        fn from(value: IntValue<'ctx>) -> Value<'ctx> {
            Value(value.0, PhantomData)
        }
    }
    impl<'ctx> Value<'ctx> {
        pub fn get_type(&self) -> Type<'ctx> {
            Type(unsafe {
                LLVMTypeOf(self.0)
            }, PhantomData)
        }

        pub fn is_constant(&self) -> bool {
            unsafe {
                LLVMIsConstant(self.0) != 0
            }
        }

        pub fn get_name(&self) -> &CStr {
            unsafe {
                let mut length: u64 = 0;
                let name = LLVMGetValueName2(self.0, &mut length as *mut _);
                let name = CStr::from_ptr(name);
                name
            }
        }

        pub fn set_name(&self, name: &str) {
            let len = name.len();
            let name = CString::new(name).unwrap();
            unsafe {
                LLVMSetValueName2(self.0, name.as_ptr(), len as u64);
            }
        }
    }

    extern "C" {
        fn LLVMAppendBasicBlock(r#Fn: LLVMValueRef, name: *const c_char) -> LLVMBasicBlockRef;
    }

    #[derive(Copy, Clone)]
    pub struct FnValue<'ctx>(LLVMValueRef, Ctx<'ctx>);
    impl<'ctx> FnValue<'ctx> {
        pub fn append_basic_block(&self, name: impl AsRef<str>) -> BasicBlock<'ctx> {
            let name = CString::new(name.as_ref()).unwrap();
            BasicBlock(unsafe {
                LLVMAppendBasicBlock(self.0, name.as_ptr())
            }, PhantomData)
        }
    }

    extern "C" {
        fn LLVMConstInt(Ty: LLVMTypeRef, N: c_ulonglong, SignExtend: LLVMBool) -> LLVMValueRef;
    }

    #[derive(Copy, Clone)]
    pub struct IntValue<'ctx>(LLVMValueRef, Ctx<'ctx>);
    impl<'ctx> IntValue<'ctx> {
        pub fn const_int(ty: IntType, value: u64, signed: bool) -> Self {
            Self(unsafe {
                LLVMConstInt(ty.0, value as _, signed as _)
            }, PhantomData)
        }
    }

    extern "C" {
        fn LLVMCreateBasicBlockInContext(ctx: LLVMContextRef, name: *const c_char) -> LLVMBasicBlockRef;
        fn LLVMGetBasicBlockName(BB: LLVMBasicBlockRef) -> *const c_char;
    }
    #[derive(Copy, Clone)]
    pub struct BasicBlock<'ctx>(LLVMBasicBlockRef, Ctx<'ctx>);
    impl<'ctx> BasicBlock<'ctx> {
        pub fn new(name: impl AsRef<str>, context: &'ctx Context) -> Self {
            let name = CString::new(name.as_ref()).unwrap();
            Self(unsafe {
                LLVMCreateBasicBlockInContext(context.0, name.as_ptr())
            }, PhantomData)
        }

        pub fn name(&self) -> &str {
            let name = unsafe {
                CStr::from_ptr(LLVMGetBasicBlockName(self.0))
            };
            name.to_str().unwrap()
        }
    }
    
    extern "C" {
        fn LLVMCreateBuilderInContext(ctx: LLVMContextRef) -> LLVMBuilderRef;
        fn LLVMDisposeBuilder(Builder: LLVMBuilderRef);
        fn LLVMPositionBuilderAtEnd(Builder: LLVMBuilderRef, Block: LLVMBasicBlockRef);
        fn LLVMBuildRet(Builder: LLVMBuilderRef, Value: LLVMValueRef);
    }
    pub struct Builder<'ctx>(LLVMBuilderRef, Ctx<'ctx>);
    impl<'ctx> Builder<'ctx> {
        pub fn new(context: &'ctx Context) -> Self {
            Self(unsafe {
                LLVMCreateBuilderInContext(context.0)
            }, PhantomData)
        }

        pub fn position_at_end(&self, block: BasicBlock<'ctx>) {
            unsafe {
                LLVMPositionBuilderAtEnd(self.0, block.0)
            }
        }

        pub fn build_return(&self, value: impl Into<Value<'ctx>>) {
            unsafe {
                LLVMBuildRet(self.0, value.into().0)
            }
        }
    }

    impl Drop for Builder<'_> {
        fn drop(&mut self) {
            unsafe {
                LLVMDisposeBuilder(self.0)
            }
        }
    }
}

use ffi::{Context, Module, IntType, FnType, Builder, BasicBlock, IntValue};

#[test]
fn test_context() {
    let context = Context::new();
    let module = Module::new("test0", &context);

    let fn_type = FnType::new(IntType::int64(&context), &mut [], false);
    let func = module.add_function("main", fn_type);

    let builder = Builder::new(&context);

    let entry = func.append_basic_block("entry");
    builder.position_at_end(entry);

    let zero = IntValue::const_int(IntType::int64(&context), 0, true);
    builder.build_return(zero);

    println!("{}", module);
}

