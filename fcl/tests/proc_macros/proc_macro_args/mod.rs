
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
>                        
quote_as_expr                            

TODO: Use `updated_attr_args()` eherevere applicable.
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

mod fn_fn_call_params;
mod fn_fn_closure_coords;
mod mod_fn;
// fn_mod
mod trait_fn;
mod impl_struct_fn;
mod trait_macro;
mod fn_init_fn;