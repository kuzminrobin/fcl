#[fcl_proc_macros::loggable]
// #[fcl_proc_macros::loggable(singlethreaded)]
pub fn main() {
    // fcl::_single_threaded_otimization!();
    // fcl::call_log_infra::THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.
    let _a = Some(0);

    use root::*;
    f();
    g();

    {
        use crate::*;
        crate::S.d();  // Expected: S::d()
        // crate::<S as Tr>.d();  // Expected: S::d()
        crate::S2.d(); // Expected: Tr::d()
        crate::S2.e();

        let s = S;
        let s2 = S2;
        let a: Vec<&dyn Tr> = vec![ &s, &s2 ];
        a[0].d();   // Expected: S::d()
        a[1].d();   // Expected: Tr::d()
    }
}


// use fcl_proc_macros::loggable;
#[fcl_proc_macros::loggable]
mod root {
    // TODO: Test: All other items at https://docs.rs/syn/latest/syn/enum.Item.html
    // Const(ItemConst)
    // Enum(ItemEnum)
    // ExternCrate(ItemExternCrate)
    // ForeignMod(ItemForeignMod)
    // Macro(ItemMacro)
    // Static(ItemStatic)
    // Struct(ItemStruct)
    // Trait(ItemTrait)
    // TraitAlias(ItemTraitAlias)
    // Type(ItemType)
    // Union(ItemUnion)
    // Verbatim(TokenStream)

    // use fcl_proc_macros::loggable;
    // use fcl::FunctionLogger;

    // mod m0; // Compiler Error: non-inline modules in proc macro input are unstable. see issue #54727
    mod m1 {}

    mod m { 
        // use fcl_proc_macros::loggable;
        // use fcl::FunctionLogger;

        fn h() {}
        pub fn i() {
            h();
        }
    }
    pub fn f() {} 
    pub fn g() {
        m::i();
    }
    // // #[fcl_proc_macros::loggable]
    // pub fn main() {
    //     fcl/*::call_log_infra */::_single_threaded_otimization!();
    //     // fcl::call_log_infra::THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.
    //     let _a = Some(0);
    //     f();
    //     g();

    //     {
    //         use crate::*;
    //         crate::S.d();  // Expected: S::d()
    //         // crate::<S as Tr>.d();  // Expected: S::d()
    //         crate::S2.d(); // Expected: Tr::d()
    //         crate::S2.e();

    //         let s = S;
    //         let s2 = S2;
    //         let a: Vec<&dyn Tr> = vec![ &s, &s2 ];
    //         a[0].d();   // Expected: S::d()
    //         a[1].d();   // Expected: Tr::d()
    //     }
    // }
}
// pub use root::*;


#[fcl_proc_macros::loggable]
trait Tr {
    fn d(&self) {} // Tr::d()
    fn e(&self);
}
struct S;
#[fcl_proc_macros::loggable]
impl Tr for S {
    fn d(&self) {   // S::d()
        // <Self as Tr>::d(self); // NOTE: Causes recursion of S::d() instead of calling Tr::d().
        // Tr::d(self);    // NOTE: Causes recursion of S::d() instead of calling Tr::d().
        // self.Tr::d();
    }  
    fn e(&self) {}
}
struct S2;
#[fcl_proc_macros::loggable]
impl Tr for S2 {    // Reuses Tr::d()
    fn e(&self) {
        Some(1).map(|val| !val );
    }
}
