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
    fn f() {} 
    fn g() {
        m::i();
    }
    pub fn main() {
        // fcl::call_log_infra::THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.
        f();
        g();
    }
}
pub use root::*;

