
/*
Find `is_traverse_stopper()` to see all the places to combine the `attr_args`.

//                                            | Attr Args Combining
//--------------------------------------------+-----------------------
quote_as_item                                   N/A
    quote_as_item_fn                            OK  fn_fn_*
        traversed_block_from_sig                N/A
            quote_as_block                      N/A
                quote_as_block_statements       N/A
                    quote_as_stmt               N/A
                        quote_as_local          N/A (w TODO, see below)
                            quote_as_init       N/A
                        quote_as_item           ^ - see above
                        quote_as_expr           v - see below
                        quote_as_stmt_macro     N/A (w TODO "What if the user has `#![feature(proc_macro_hygiene)]`")
                            quote_as_macro      N/A, ignores `attr_args`.
    quote_as_item_impl                          OK. TODO: Tests
        quote_as_impl_item                      N/A
            quote_as_impl_item_fn               OK. TODO: Test
            quote_as_impl_item_macro            TODO: Implement
    quote_as_item_macro                         OK  trait_macro.rs, TODO: {impl, impl_for, mod, fn, closure?}_macro.rs
        quote_as_item_macro_rules_def           N/A trait_macro.rs, TODO: {impl, impl_for, mod, fn, closure?}_macro.rs
        quote_as_item_macro_rules_invocation    ? (Code: `// TODO: What about combining `attr_args`?`)
    quote_as_item_mod                           OK mod_fn.rs, TODO: mod_macro.rs
        quote_as_item                           ^ - see above
    quote_as_item_static                        OK, TODO: Test
        quote_as_expr                           v - see below
    quote_as_item_trait                         OK, trait_fn.rs, trait_macro.rs. TODO: trait_*
        quote_as_trait_item                     N/A
            quote_as_trait_item_const           OK, TODO: Test
            quote_as_trait_item_fn              OK, trait_fn
                traversed_block_from_sig        ^ - see above
            quote_as_trait_item_macro_rules_invocation
                                                OK, trait_macro.rs
// TODO: 
// * Consider `update_passed` -> `attrs_have_non_loggable` | `non_loggable_found`.
// *           `has_loggable` -> `attrs_have_loggable` | `loggable_found`.
quote_as_expr
    quote_as_expr_array                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_assign                        OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_async                         OK, TODO: Test.
        quote_as_block                          ^ - see above
    quote_as_expr_await                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_binary                        OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_block                         OK, TODO: Test.
        quote_as_block                          ^ - see above
    quote_as_expr_break                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_call                          OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_cast                          OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_closure                       OK, TODO: Test.
        quote_as_expr                           ^ - see above
    // quote_as_expr_const
    // quote_as_expr_continue
    quote_as_expr_field                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_for_loop                      OK, TODO: Test.
        quote_as_expr                           ^ - see above
        quote_as_loop_block                     N/A
    quote_as_expr_group                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_if                            OK, TODO: Test.
        quote_as_expr                           ^ - see above
        quote_as_block                          ^ - see above
        quote_as_expr                           ^ - see above
    quote_as_expr_index                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    // quote_as_expr_infer

    quote_as_expr_let                           OK, TODO: Test.
        quote_as_expr                           ^ - see above
    // quote_as_expr_lit
    quote_as_expr_loop                          OK, TODO: Test.
        quote_as_loop_block                     ^ - see above
    quote_as_expr_macro                         Postponed
        quote_as_macro                          ^ - see above
    quote_as_expr_match                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
        quote_as_arm                            N/A, The `match` arms cannot be `#[loggable]` and `#[non_loggable]`
            quote_as_expr                       ^ - see above
    quote_as_expr_method_call                   OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_paren                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_path                          N/A, no traverse.
    quote_as_expr_range                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_raw_addr                      OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_reference                      OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_repeat                        OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_return                        OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_struct                        OK, TODO: Test.
        quote_as_field_value                    N/A, Cannot be `#[[non_]loggable]`.
            quote_as_expr                       ^ - see above
        quote_as_expr                           ^ - see above
    quote_as_expr_try                           OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_try_block                     OK, TODO: Test.
        quote_as_block                          ^ - see above
    quote_as_expr_tuple                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_unary                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
    quote_as_expr_unsafe                        OK, TODO: Test.
        quote_as_block                          ^ - see above
    quote_as_expr_while                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
        quote_as_loop_block                     N/A
    quote_as_expr_yield                         OK, TODO: Test.
        quote_as_expr                           ^ - see above
>
    Commi

TODO: Use `updated_attr_args()` wherever applicable.
 */

// params
// closure_coords
// prefix (low priority)

// outer             | both: closure_coords, params
//   |               | 
//   V      inner -> | mod | trait | impl | struct | impl trait | fn             | static| macro
// ------------------------------------------------------------------------      ----------------
// mod               |     |       |      |        |            | mod_fn         |       |      
// trait             |     |       |      |        |            | trait_fn       |       | trait_macro
// impl struct       |     |       |      |        |            |                |       |       
// impl trait        |     |       |      |        |            |                |       |      
// fn                |     |       |      |        |            | fn_fn_*        |       |      
//                   |     |       |      |        |            | fn_init_fn     |       |
// static            |     |       |      |        |            |                |       |
// -----------------------------------------------------------------------------------------------
// Array             |     |       |      |        |            |                |       |
// Assign            |     |       |      |        |            |                |       |
// Async             |     |       |      |        |            |                |       |
// Await             |     |       |      |        |            |                |       |
// Binary            |     |       |      |        |            |                |       |
// Block             |     |       |      |        |            | .              |       |
// Break             |     |       |      |        |            |                |       |
// Call              |     |       |      |        |            |                |       |
// Cast              |     |       |      |        |            |                |       |
// Closure           |     |       |      |        |            |                |       |
// Const             |     |       |      |        |            |                |       |
// Continue          |     |       |      |        |            |                |       |
// Field             |     |       |      |        |            |                |       |
// ForLoop           |     |       |      |        |            |                |       |
// Group             |     |       |      |        |            |                |       |
// If                |     |       |      |        |            |                |       |
// Index             |     |       |      |        |            |                |       |
// Infer             |     |       |      |        |            |                |       |
// Let               |     |       |      |        |            |                |       |
// Lit               |     |       |      |        |            |                |       |
// Loop              |     |       |      |        |            |                |       |
// Macro             |     |       |      |        |            |                |       |
// Match             |     |       |      |        |            |                |       |
// MethodCall        |     |       |      |        |            |                |       |
// Paren             |     |       |      |        |            |                |       |
// Path              |     |       |      |        |            |                |       |
// Range             |     |       |      |        |            |                |       |
// RawAddr           |     |       |      |        |            |                |       |
// Reference         |     |       |      |        |            |                |       |
// Repeat            |     |       |      |        |            |                |       |
// Return            |     |       |      |        |            |                |       |
// Struct            |     |       |      |        |            |                |       |
// Try               |     |       |      |        |            |                |       |
// TryBlock          |     |       |      |        |            |                |       |
// Tuple             |     |       |      |        |            |                |       |
// Unary             |     |       |      |        |            |                |       |
// Unsafe            |     |       |      |        |            |                |       |
// While             |     |       |      |        |            |                |       |
// Yield             |     |       |      |        |            |                |       |
// Verbatim          |     |       |      |        |            |                |       |

#[cfg(feature = "closure_coords_logging")]
/// NOTE: Cannot be a function since participates in a string literal generation.
macro_rules! closure_coords {
    () => { "0,0:0,0" }
}
#[cfg(not(feature = "closure_coords_logging"))]
/// NOTE: Cannot be a function since participates in a string literal generation.
macro_rules! closure_coords {
    () => { "" }
}

#[cfg(feature = "closure_coords_logging")]
/// NOTE: Cannot be a function since participates in a string literal generation.
macro_rules! dots {
    () => { ".." }
}
#[cfg(not(feature = "closure_coords_logging"))]
/// NOTE: Cannot be a function since participates in a string literal generation.
macro_rules! dots {
    () => { "" }
}

mod fn_fn_call_params;
// #[cfg(feature = "closure_coords_logging")]
mod fn_fn_closure_coords;
mod mod_fn;
// fn_mod
mod trait_fn;
mod impl_struct_fn;
mod trait_macro;
mod fn_init_fn;