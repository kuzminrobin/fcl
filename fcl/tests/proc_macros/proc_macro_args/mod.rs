


// params
// closure_coords
// prefix (low priority)

// outer             | both: closure_coords, params
//   |               | 
//   V      inner -> | mod | trait | impl | struct | impl trait | fn             | static
// ------------------------------------------------------------------------      ----------------
// mod               |     |       |      |        |            | both           |       |      mod/fn
// trait             |     |       |      |        |            | both           |       |      trait/fn
// impl struct       |     |       |      |        |            | .              |       | 
// impl trait        |     |       |      |        |            |                |       |
// fn                |     |       |      |        |            | both           |       |      fn/fn
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