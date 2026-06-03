/// The name of a procedural macro that does the FCL instrumentation.
pub const LOGGABLE_MACRO_NAME: &str = "loggable";
/// The name of a crate that exposes the pocedural macros that handle the FCL instrumentation.
pub const CRATE_NAME: &str = "fcl_proc_macros";
/// The prefix added to the last path segment of a declarative macro name if that macro (invocation and definitions) is made `#[loggable]`.
///
/// ### Examples
/// ```ignore
/// macro_rules! users_macro {  // Declarative macro definition.
///     () => {}
/// }
///
/// users_macro() // Invocation.
/// ```
/// If `#[loggable]` is applied to the macro (must be done for both the definition and all the invocations)
/// ```ignore
/// #[loggable]
/// macro_rules! users_macro {  // Declarative macro definition.
///     () => {}
/// }
///
/// #[loggable]
/// users_macro(); // Invocation.
/// ```
/// then the macro name "users_macro" will be prefixed with `LOGGABLE_MACRO_NAME_PREFIX`
/// (at the moment of writing the new name will be "loggable_macro_users_macro").
pub const LOGGABLE_MACRO_NAME_PREFIX: &str = "loggable_macro_";
/// An intermediate name used for instrumenting the 
/// [MacroTranscriber](https://doc.rust-lang.org/reference/macros-by-example.html#grammar-MacroTranscriber) 
/// of the `#[loggable]` declarative macros.
/// 
/// See (TODO: mdBook link - "The Declarative (`macro_rules`) Macros That Are `#[loggable]`").
pub const INTERMEDIATE_FN_NAME_FOR_MACRO_TRANSCRIBER: &str = "loggable_block_contents";

/// Macro argument (`__`) that is passed to the declarative macro parameter `$prefix` of the macro whose name starts with
/// [`LOGGABLE_MACRO_NAME_PREFIX`] if the argument is not provided from outside.
macro_rules! EMPTY_PREFIX_SUBSTITUTE {
    () => {
        quote! { __ }
    };
}
