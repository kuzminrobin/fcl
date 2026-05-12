
/*
// Attr Args Combining
quote_as_item                                   N/A
    quote_as_item_fn                            OK  fn_fn_*
        traversed_block_from_sig                N/A
            quote_as_block                      N/A
                quote_as_block_statements       N/A
                    quote_as_stmt               N/A
                        quote_as_local          TODO
>                        
                            quote_as_init
                        quote_as_item   ^
                        quote_as_expr   v
                        quote_as_stmt_macro
                            quote_as_macro
    quote_as_item_impl
    quote_as_item_macro
    quote_as_item_mod
    quote_as_item_static
    quote_as_item_trait
quote_as_expr                            
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