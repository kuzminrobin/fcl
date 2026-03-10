use std::str::FromStr;

use quote::quote;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma, *};

/// Suppresses the automatic recursive instrumentation of an item as `#[loggable]`.
///
/// For example, if a struct implementation is marked as `#[loggable]` then
/// the associated funtions defined in that struct implementation will automatically be
/// instrumented as `#[loggable]`. The `#[non_loggable]` attribute added to an
/// associated funtion suppresses the instrumentation for that function.
///
/// # Examples
/// ```compile_fail, E0432, E0433, E0599
/// use fcl_proc_macros::{loggable, non_loggable};
///
/// struct MyStruct;
///
/// #[loggable] // Automatically recursively instruments the nested items.
/// impl MyStruct {
///     // This associated function gets instrumented automatically:
///     fn assoc_func_loggable(&self) {}
///
///     #[non_loggable] // Suppresses the automatic instrumentation during recursion.
///     fn assoc_func_non_loggable(&self) { // This associated function doesn't get instrumented.
///         let _v = Some(1).map(
///             |val| val + 1   // This closure doesn't get instrumented.
///         );
///
///         #[loggable] // Automatically recursively instruments the item.
///         fn local_func() { // This function gets instrumented recursively.
///             let _v2 = Some(2).map(
///                 |val| val + 2   // This closure gets automatically instrumented recursively.
///                 // TODO: Test this.
///             );
///         }
///
///         local_func();
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn non_loggable(
    _attr_args: proc_macro::TokenStream,
    attributed_item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    attributed_item
}

/// Instruments the item and nested definitions recursively to be logged
/// by the Function Call Logger (FCL).
/// ### Examples
/// ```compile_fail, E0432, E0433, E0599
/// use fcl_proc_macros::{loggable, non_loggable};
///
/// #[loggable] // Automatically instruments the nested items.
/// mod my_module {
///
///     struct MyStruct;
///
///     impl MyStruct {
///         fn assoc_func_loggable(&self) { // This associated function gets instrumented.
///             let v = Some(1).map(
///                 |val| val + 1   // This closure gets instrumented.
///             );
///         }
///         
///         #[non_loggable] // Suppresses the automatic instrumentation recursively.
///         fn assoc_func_non_loggable(&self) {} // This associated function and its local entities don't get instrumented.
///     }
/// }
/// ```
/// ## Parameters
/// 
/// ### `log_params` (default, optional)
/// Log the parameters in the annotated entity and its local entities recursively.
/// 
/// ### `skip_params` (optional)
/// Skip the parameters logging in the annotated entity and its local entities recursively.  
/// 
/// If the function has no prameters then its parameters block is logged as `()`, otherwise `(..)`.
/// 
/// ### Examples 
/// ```ignore
/// #[loggable(skip_params)] // Skip the parameter logging for function `f()` and its local entities recursively.
/// fn f(b: bool) {} // Logs: `f(..) {}`. The parameter `b` is not logged.
/// 
/// #[loggable(log_params)] // Log the parameters of the functions and closures inside of module `m` recursively.
/// mod m {
///     fn f(b: bool) {}    // Log example: `m::f(b: true) {}`. The parameter `b` is logged.
/// }
/// ```
/// `#[loggable]` has the same effect as `#[loggable(log_params)]`, i.e., the parameters are logged by default.
/// 
/// ### `prefix` (optional)
/// Is unlikely to be used by the user.
/// 
/// Sets the name prefix for the annotated entity recursively.
/// 
/// #### Examples
/// ```ignore
/// #[loggable(prefix = A)]
/// fn f() { 
///     fn local_func() {}  // Define a local function.
/// 
///     local_func();       // Call the local function.
/// }   
/// // FCL Log: 
/// A::f() {                    // Is prefixed with "A::".
///   A::f()::local_func() {}   // Is prefixed with "A::".
/// } // A::f()
/// 
/// #[loggable(prefix = my_module::<MyStruct as MyPureTrait>::B)]
/// fn f() {}   
/// // FCL Log: 
/// my_module::<MyStruct as MyPureTrait>::B::f() {} // Is prefixed with "my_module::<MyStruct as MyPureTrait>::B::".
/// ```
#[proc_macro_attribute]
pub fn loggable(
    attr_args_ts: proc_macro::TokenStream,
    attributed_item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr_args = parse_macro_input!(attr_args_ts as AttrArgs); // Handles the compilation errors appropriately (checked).
    let output = {
        if let Ok(item) = syn::parse::<Item>(attributed_item.clone()) {
            quote_as_item(&item, &attr_args)
        } else if let Ok(expr) = syn::parse::<Expr>(attributed_item.clone()) {
            quote_as_expr(&expr, None, &attr_args)
        } else {
            let closure_w_opt_comma = parse_macro_input!(attributed_item as ExprClosureWOptComma); // Handles the compilation errors appropriately.
            quote_as_expr_closure(&closure_w_opt_comma.closure, &attr_args)
        }
    };
    let ret_val = quote! { #output };
    ret_val.into()
}

/// Removes spaces from a string, except around 'as' (in framgents like "\<MyType as MyTrait>").
///
/// Returns a copy of an argument with spaces removed, except around 'as'.
///
/// NOTE: If the argument contains sequences of '$as$', those will be replaced with ' as '.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(
///     remove_spaces(&"<MyType as MyTrait> :: my_func"),
///     "<MyType as MyTrait>::my_func" // The spaces around '::' are removed, but around 'as' are not.
/// );
/// ```
fn remove_spaces(s: &str) -> String {
    // Preserve spaces in fragments like `<MyType as MyTrait>`.
    let tmp_str: String = s
        .replace(" as ", "$as$")
        .chars()
        .filter(|ch| *ch != ' ')
        .collect();
    tmp_str.replace("$as$", " as ")
}

fn quote_as_expr_array(
    expr_array: &ExprArray,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprArray { // [a, b, c, d]
        attrs, //: Vec<Attribute>,
        // bracket_token, //: Bracket,
        elems, //: Punctuated<Expr, Comma>,
        .. // bracket_token
    } = expr_array;
    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_array };
        }
    }
    let elems = {
        let mut traversed_elems = quote! {};
        for elem in elems {
            let traversed_elem = quote_as_expr(elem, None, attr_args);
            traversed_elems = quote! { #traversed_elems #traversed_elem , };
        }
        traversed_elems
    };

    quote! { #(#attrs)* [ #elems ] }
}

fn quote_as_expr_assign(
    expr_assign: &ExprAssign,
    attr_args: &AttrArgs
) -> proc_macro2::TokenStream {
    // a = compute()
    let ExprAssign {
        attrs,    //: Vec<Attribute>,
        left,     //: Box<Expr>,
        eq_token, //: Eq,
        right,    //: Box<Expr>,
    } = expr_assign;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_assign };
        }
    }
    let left = quote_as_expr(left, None, attr_args);
    let right = quote_as_expr(right, None, attr_args);
    quote! { #(#attrs)* #left #eq_token #right }
}
fn quote_as_expr_async(
    expr_async: &ExprAsync,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprAsync {
        // async { ... }
        attrs,       //: Vec<Attribute>,
        async_token, //: Async,
        capture,     //: Option<Move>,
        block,       //: Block,
    } = expr_async;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_async };
        }
    }
    let block = quote_as_block(block, attr_args);
    quote! { #(#attrs)* #async_token #capture #block }
}
fn quote_as_expr_await(
    expr_await: &ExprAwait,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprAwait {
        // fut.await
        attrs,       //: Vec<Attribute>,
        base,        //: Box<Expr>,
        dot_token,   //: Dot,
        await_token, //: Await,
    } = expr_await;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_await };
        }
    }
    let base = quote_as_expr(base, None, attr_args);
    quote! { #(#attrs)* #base #dot_token #await_token }
}
fn quote_as_expr_binary(
    expr_binary: &ExprBinary,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprBinary {
        // `a + b`, `a += b`
        attrs, //: Vec<Attribute>,
        left,  //: Box<Expr>,
        op,    //: BinOp,
        right, //: Box<Expr>,
    } = expr_binary;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_binary };
        }
    }
    let left = quote_as_expr(left, None, attr_args);
    let right = quote_as_expr(right, None, attr_args);
    quote! { #(#attrs)* #left #op #right }
}
fn quote_as_expr_block(
    expr_block: &ExprBlock,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprBlock {
        attrs, //: Vec<Attribute>,
        label, //: Option<Label>,
        block, //: Block,
    } = expr_block;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_block };
        }
    }
    let block = quote_as_block(block, attr_args);
    quote! { #(#attrs)* #label #block }
}
fn quote_as_expr_break(
    expr_break: &ExprBreak,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprBreak {
        attrs,       //: Vec<Attribute>,
        break_token, //: Break,
        label,       //: Option<Lifetime>,
        expr,        //: Option<Box<Expr>>,
    } = expr_break;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_break };
        }
    }
    let expr = expr.as_ref().map(|expr| quote_as_expr(expr, None, attr_args));
    quote! { #(#attrs)* #break_token #label #expr }
}
fn quote_as_expr_call(
    expr_call: &ExprCall,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprCall {
        attrs, //: Vec<Attribute>,
        func, //: Box<Expr>,
        // paren_token, //: Paren,
        args, //: Punctuated<Expr, Comma>,
        .. // paren_token
    } = expr_call;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_call };
        }
    }
    let mut is_print_func_name = false;
    let func = quote_as_expr(func, Some(&mut is_print_func_name), attr_args);
    let args = {
        let mut traversed_args = quote! {};
        for arg in args {
            let traversed_arg = quote_as_expr(arg, None, attr_args);
            traversed_args = quote! { #traversed_args #traversed_arg, }
        }
        traversed_args
    };
    let mut ret_val = quote! { #(#attrs)* #func ( #args ) };
    if is_print_func_name {
        #[cfg(feature = "singlethreaded")]
        let thread_logger_access = quote! {
            use std::borrow::BorrowMut;
            fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                logger.borrow_mut().borrow_mut().maybe_flush();
            })
        };
        #[cfg(not(feature = "singlethreaded"))]
        let thread_logger_access = quote! {
            fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                logger.borrow_mut().maybe_flush();
            })
        };
        ret_val = quote! {
            #thread_logger_access;
            #ret_val
        }
    };
    ret_val
}
fn quote_as_expr_cast(
    expr_cast: &ExprCast,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    // foo as f64
    let ExprCast {
        attrs,    //: Vec<Attribute>,
        expr,     //: Box<Expr>,
        as_token, //: As,
        ty,       //: Box<Type>,
    } = expr_cast;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_cast };
        }
    }
    let expr = quote_as_expr(expr, None, attr_args);
    quote! { #(#attrs)* #expr #as_token #ty }
}
fn quote_as_expr_closure(
    expr_closure: &ExprClosure,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprClosure {
        attrs,      //: Vec<Attribute>,
        lifetimes,  //: Option<BoundLifetimes>,
        constness,  //: Option<Const>,
        movability, //: Option<Static>,
        asyncness,  //: Option<Async>,
        capture,    //: Option<Move>,
        or1_token,  //: Or,
        inputs,     //: Punctuated<Pat, Comma>,
        or2_token,  //: Or,
        output,     //: ReturnType,
        body,       //: Box<Expr>,
    } = expr_closure;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_closure };
        }
    }
    // Get the token stream of {{param names and values} optional string}:
    let input_vals = 
        if inputs.is_empty() {
            quote!{ None }
        } else { 
            match attr_args.params_logging {
                ParamsLogging::Log => {
                    let mut param_format_str = String::new();
                    let mut param_list = quote! {};
                    for (idx, input_pat) in inputs.iter().enumerate() {
                        if idx != 0 {
                            param_format_str.push_str(", ");
                        }
                        update_param_data_from_pat(input_pat, &mut param_format_str, &mut param_list);
                    }
                    quote! { Some(format!(#param_format_str, #param_list)) }
                }
                ParamsLogging::Skip => {
                    quote!{ Some(String::from("..")) }
                }
            }
        };
    
    // Closure name:
    let (start_line, start_col) = {
        let proc_macro2::LineColumn { line, column } = or1_token.span().start();
        (line, column + 1)
    };
    let (end_line, end_col) = {
        let proc_macro2::LineColumn { line, column } = body.span().end();
        (line, column)
    };
    let coords_str = format!("{},{}:{},{}", start_line, start_col, end_line, end_col);
    let coords_ts = {
        let to_ts_res = proc_macro2::TokenStream::from_str(&coords_str);
        match to_ts_res {
            Ok(ts) => quote! { #ts },
            Err(_lex_err) => quote! { #coords_str },
        }
    };
    let mut log_closure_name_ts = quote! { closure{#coords_ts} };
    if !attr_args.prefix.is_empty() {
        let prefix = &attr_args.prefix;
        log_closure_name_ts = quote! { #prefix::#log_closure_name_ts }
    }
    let log_closure_name_str = remove_spaces(&log_closure_name_ts.to_string());
    let attr_args = AttrArgs { 
        prefix: log_closure_name_ts,
        params_logging: attr_args.params_logging,
    };

    let body = { quote_as_expr(&**body, None, &attr_args) };

    let logging_is_on = quote! {
        logger.borrow()
    };
    #[cfg(feature = "singlethreaded")]
    let logging_is_on = quote! {
        #logging_is_on.borrow()
    };
    let logging_is_on = quote! {
        #logging_is_on.logging_is_on()
    };

    // Return the token stream of the instrumented closure:
    quote! {
        #(#attrs)*
        #lifetimes #constness #movability #asyncness #capture
        #or1_token #inputs #or2_token #output
        {
            use fcl::{CallLogger, MaybePrint};

            let ret_val = fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| { // NOTE: The `logger` is used in `logging_is_on`.
                // NOTE: Borrows the params, has to be in front of the `body`
                // that moves the params to the `body` closure.
                //
                // At run time get the parameter names and values string:
                let param_val_str = #input_vals;

                // Get the body as a closure (to be executed later):
                let mut body = move || { #body };

                // If logging is off then do nothing
                // except executing the body and returning the value:
                if ! #logging_is_on {
                    return body();
                }
                // Else (logging is on):

                // Log the call, like `f()::closure{3,7:5:11}(param: true) {`:
                let mut callee_logger = fcl::CalleeLogger::new(
                    #log_closure_name_str, param_val_str);

                // Execute the body and catch the return value:
                let ret_val = body();

                // Uncondititonally tell the `callee_logger` what closure returns,
                // since if the closure's return type is not specified explicitly
                // then the return type is determined with the type inference
                // which is not available now at pre-compile (preprocessing) time.
                // In other words, at pre-compile time we don't know for sure
                // if {the closure return type is the unit type `()` and the return value logging can be skipped}.
                let ret_val_str = format!("{}", ret_val.maybe_print());
                callee_logger.set_ret_val(ret_val_str);

                // Log the return, like `} // f()::closure{3,7:5:11}() -> 5.`,
                // in the `callee_logger` destructor and return the value:
                ret_val
            });
            ret_val
        }
    }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_expr_const(expr_const: &ExprConst, _attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     quote!{ #expr_const }
//     // let ExprConst {
//     //     attrs, //: Vec<Attribute>,
//     //     const_token, //: Const,
//     //     block, //: Block,
//     // } = expr_const;
//     // let block = quote_as_expr_block(block, _attr_args);
//     // quote!{ #(#attrs)* #const_token #block }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_expr_continue(expr_continue: &ExprContinue, _attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     quote!{ #expr_continue }   // A `continue`, with an optional label.
// }
fn quote_as_expr_field(
    expr_field: &ExprField,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprField {
        attrs,     //: Vec<Attribute>,
        base,      //: Box<Expr>,
        dot_token, //: Dot,
        member,    //: Member,
    } = expr_field;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_field };
        }
    }
    let base = quote_as_expr(&**base, None, attr_args);
    quote! { #(#attrs)* #base #dot_token #member }
}
fn quote_as_expr_for_loop(
    expr_for_loop: &ExprForLoop,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprForLoop {
        attrs,     //: Vec<Attribute>,
        label,     //: Option<Label>,
        for_token, //: For,
        pat,       //: Box<Pat>,
        in_token,  //: In,
        expr,      //: Box<Expr>,
        body,      //: Block,
    } = expr_for_loop;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_for_loop };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    let body = quote_as_loop_block(body, attr_args);
    quote! {
        {
            let loop_result = { // At the moment of writing the unit value `()`
                // is the only known possible value returnable by `for` loop.
                #(#attrs)* #label #for_token #pat #in_token #expr #body
            };

            fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger|
                logger.borrow_mut().log_loop_end());

            loop_result
        }
    }
}
fn quote_as_expr_group(
    expr_group: &ExprGroup,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprGroup {
        attrs, //: Vec<Attribute>,
        // group_token, //: Group,
        expr, //: Box<Expr>,
        .. // group_token
    } = expr_group;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_group };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    // the trait bound `syn::token::Group: quote::ToTokens` is not satisfied
    quote! { { #(#attrs)* #expr } }
}
fn quote_as_expr_if(
    expr_if: &ExprIf,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprIf {
        attrs,       //: Vec<Attribute>,
        if_token,    //: If,
        cond,        //: Box<Expr>,
        then_branch, //: Block,
        else_branch, //: Option<(Else, Box<Expr>)>,
    } = expr_if;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_if };
        }
    }
    let cond = quote_as_expr(&**cond, None, attr_args);
    let then_branch = quote_as_block(then_branch, attr_args);
    let else_branch = else_branch.as_ref().map(|(else_token, expr)| {
        let expr = quote_as_expr(&**expr, None, attr_args);
        quote! { #else_token #expr }
    });
    quote! { #(#attrs)* #if_token #cond #then_branch #else_branch }
}
fn quote_as_expr_index(
    expr_index: &ExprIndex,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprIndex {
        attrs, //: Vec<Attribute>,
        expr, //: Box<Expr>,
        // bracket_token, //: Bracket,
        index, //: Box<Expr>,
        .. // bracket_token
    } = expr_index;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_index };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    let index = quote_as_expr(&**index, None, attr_args);
    quote! { #(#attrs)* #expr [ #index ] }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_expr_infer(expr_infer: &ExprInfer, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     quote!{ #expr_infer }
// }
fn quote_as_expr_let(
    expr_let: &ExprLet,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprLet {
        attrs,     //: Vec<Attribute>,
        let_token, //: Let,
        pat,       //: Box<Pat>,
        eq_token,  //: Eq,
        expr,      //: Box<Expr>,
    } = expr_let;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_let };
        }
    }
    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let pat = quote_as_pat(&**pat, attr_args);
    let expr = quote_as_expr(&**expr, None, attr_args);
    quote! { #(#attrs)* #let_token #pat #eq_token #expr }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_expr_lit(expr_lit: &ExprLit, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     quote!{ #expr_lit }
// }
fn quote_as_expr_loop(
    expr_loop: &ExprLoop,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprLoop {
        attrs,      //: Vec<Attribute>,
        label,      //: Option<Label>,
        loop_token, //: Loop,
        body,       //: Block,
    } = expr_loop;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_loop };
        }
    }
    let body = quote_as_loop_block(body, attr_args);
    quote! {
        // // Ret val for `loop` has been deprioritized since it requires extra
        // // refactoring for the case of a (removed) loopbody with no nested calls.
        // {
        //     let ret_val =
                #(#attrs)* #label #loop_token #body
        //     ;

        //     let ret_val_str = format!("{}", ret_val.maybe_print());
        //     fcl::call_log_infra::THREAD_LOGGER.with(|thread_logger| {
        //         thread_logger.borrow_mut().set_loop_ret_val(ret_val_str);
        //     });
        //     ret_val
        // }
    }
}
fn quote_as_arm(arm: &Arm, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let Arm {
        attrs,           //: Vec<Attribute>,
        pat,             //: Pat,
        guard,           //: Option<(If, Box<Expr>)>,
        fat_arrow_token, //: FatArrow,
        body,            //: Box<Expr>,
        comma,           //: Option<Comma>,
    } = arm;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #arm };
        }
    }
    let guard = guard.as_ref().map(|(if_token, expr)| {
        let expr = quote_as_expr(expr, None, attr_args);
        quote! { #if_token #expr }
    });
    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // guard
    let body = quote_as_expr(&**body, None, attr_args);
    quote! { #(#attrs)* #pat #guard #fat_arrow_token #body #comma }
}
fn quote_as_macro(
    macro_: &Macro,
    maybe_flush_invocation: &mut proc_macro2::TokenStream,
    _attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let Macro {
        path, //: Path,
        // bang_token, //: Not,
        // delimiter, //: MacroDelimiter,
        // tokens, //: TokenStream,
        .. // All others.
    } = macro_;
    if let Some(macro_name) = path.segments.last() {
        if &macro_name.ident.to_string() == &"println"
            || &macro_name.ident.to_string() == &"print"
            || &macro_name.ident.to_string() == &"eprintln"
            || &macro_name.ident.to_string() == &"eprint"
        {
            #[cfg(feature = "singlethreaded")]
            let thread_logger_access = quote! {
                fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                    logger.borrow_mut().borrow_mut().maybe_flush();
                })
            };
            #[cfg(not(feature = "singlethreaded"))]
            let thread_logger_access = quote! {
                fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                    logger.borrow_mut().maybe_flush();
                })
            };

            *maybe_flush_invocation = quote! {
                #thread_logger_access;
            }
        }
    }
    quote! { #macro_ }
}
fn quote_as_expr_macro(
    expr_macro: &ExprMacro,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprMacro {
        attrs, //: Vec<Attribute>,
        mac,   //: Macro,
    } = expr_macro;
    let mut maybe_flush_invocation = quote! {};
    let mac = quote_as_macro(&mac, &mut maybe_flush_invocation, attr_args);
    quote! {
        {
            #maybe_flush_invocation;
            #(#attrs)* #mac
        }
    }
}
fn quote_as_expr_match(
    expr_match: &ExprMatch,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprMatch {
        attrs, //: Vec<Attribute>,
        match_token, //: Match,
        expr, //: Box<Expr>,
        // brace_token, //: Brace,
        arms, //: Vec<Arm>,
        .. // brace_token
    } = expr_match;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_match };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    let mut traveresed_arms = quote! {};
    for arm in arms {
        let traversed_arm = quote_as_arm(arm, attr_args);
        traveresed_arms = quote! { #traveresed_arms #traversed_arm }
    }
    quote! { #(#attrs)* #match_token #expr { #traveresed_arms } }
}
fn quote_as_expr_method_call(
    expr_method_call: &ExprMethodCall,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprMethodCall { // x.foo::<T>(a, b)
        attrs, //: Vec<Attribute>,
        receiver, //: Box<Expr>,
        dot_token, //: Dot,
        method, //: Ident,
        turbofish, //: Option<AngleBracketedGenericArguments>,
        // paren_token, //: Paren,
        args, //: Punctuated<Expr, Comma>,
        .. // paren_token
    } = expr_method_call;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_method_call };
        }
    }
    let receiver = quote_as_expr(&**receiver, None, attr_args);
    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let turbofish = match turbofish {
    //     Some(angle_bracketed_generic_arguments) =>
    //         Some(quote_as_angle_bracketed_generic_arguments(angle_bracketed_generic_arguments, attr_args)),
    //     _ => turbofish
    // };
    let mut traversed_args = quote! {};
    for arg in args {
        let traversed_arg = quote_as_expr(arg, None, attr_args);
        traversed_args = quote! { #traversed_args #traversed_arg, }
    }
    quote! { #(#attrs)* #receiver #dot_token #method #turbofish ( #traversed_args ) }
}
fn quote_as_expr_paren(
    expr_paren: &ExprParen,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprParen { // A parenthesized expression: `(a + b)`.
        attrs, //: Vec<Attribute>,
        // paren_token, //: Paren,
        expr, //: Box<Expr>,
        .. // paren_token
    } = expr_paren;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_paren };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    quote! { #(#attrs)* ( #expr ) }
}
fn quote_as_expr_path(
    expr_path: &ExprPath,
    is_print_func_name: Option<&mut bool>,
    _attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprPath {
        // attrs, //: Vec<Attribute>,
        // qself, //: Option<QSelf>,
        path, //: Path,
        .. // attrs, qself
    } = expr_path;

    if let Some(name) = path.segments.last() {
        let name = name.ident.to_string();
        if &name == &"_print" || &name == &"_eprint" {
            if let Some(is_print_func_name) = is_print_func_name {
                *is_print_func_name = true;
            }
        }
    }
    quote! { #expr_path }
}
fn quote_as_expr_range(
    expr_range: &ExprRange,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprRange {
        attrs,  //: Vec<Attribute>,
        start,  //: Option<Box<Expr>>,
        limits, //: RangeLimits,
        end,    //: Option<Box<Expr>>,
    } = expr_range;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_range };
        }
    }
    let start = start
        .as_ref()
        .map(|start| quote_as_expr(&**start, None, attr_args));
    let end = end.as_ref().map(|end| quote_as_expr(&**end, None, attr_args));
    quote! { #(#attrs)* #start #limits #end }
}
fn quote_as_expr_raw_addr(
    expr_raw_addr: &ExprRawAddr,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprRawAddr {
        attrs,      //: Vec<Attribute>,
        and_token,  //: And,
        raw,        //: Raw,
        mutability, //: PointerMutability,
        expr,       //: Box<Expr>,
    } = expr_raw_addr;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_raw_addr };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    quote! { #(#attrs)* #and_token #raw #mutability #expr }
}
fn quote_as_expr_reference(
    expr_reference: &ExprReference,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprReference {
        attrs,      //: Vec<Attribute>,
        and_token,  //: And,
        mutability, //: Option<Mut>,
        expr,       //: Box<Expr>,
    } = expr_reference;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_reference };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    quote! { #(#attrs)* #and_token #mutability #expr }
}
fn quote_as_expr_repeat(
    expr_repeat: &ExprRepeat,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprRepeat { // [0u8; N]
        attrs, //: Vec<Attribute>,
        // bracket_token, //: Bracket,
        expr, //: Box<Expr>,
        semi_token, //: Semi,
        len, //: Box<Expr>,
        .. // bracket_token
    } = expr_repeat;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_repeat };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    let len = quote_as_expr(&**len, None, attr_args);
    quote! { #(#attrs)* [ #expr #semi_token #len ] }
}
fn quote_as_expr_return(
    expr_return: &ExprReturn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprReturn {
        attrs,        //: Vec<Attribute>,
        return_token, //: Return,
        expr,         //: Option<Box<Expr>>,
    } = expr_return;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_return };
        }
    }
    let expr = expr
        .as_ref()
        .map(|expr| quote_as_expr(&**expr, None, attr_args));
    quote! { #(#attrs)* #return_token #expr }
}
fn quote_as_field_value(
    field: &FieldValue,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let FieldValue {
        attrs,       //: Vec<Attribute>,
        member,      //: Member,
        colon_token, //: Option<Colon>,
        expr,        //: Expr,
    } = field;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #field };
        }
    }
    let expr = quote_as_expr(expr, None, attr_args);
    quote! { #(#attrs)* #member #colon_token #expr }
}
fn quote_as_expr_struct(
    expr_struct: &ExprStruct,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprStruct { // S { a: 1, b: 1, ..rest }
        attrs, //: Vec<Attribute>,
        qself, //: Option<QSelf>,
        path, //: Path,
        // brace_token, //: Brace,
        fields, //: Punctuated<FieldValue, Comma>,
        dot2_token, //: Option<DotDot>,
        rest, //: Option<Box<Expr>>,
        .. // brace_token
    } = expr_struct;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_struct };
        }
    }

    // `quote!{ #qself }`: Error: the trait bound `syn::QSelf: quote::ToTokens` is not satisfied
    // WARNING: The interpretation of {qself and path} combination below is questionable.
    // https://docs.rs/syn/latest/syn/struct.ExprPath.html#structfield.qself
    // https://docs.rs/syn/latest/syn/struct.QSelf.html
    // https://doc.rust-lang.org/reference/paths.html#qualified-paths
    let qself_and_apth = {
        match qself {
            Some(qself) => {
                let QSelf {
                    ty, //: Box<Type>,
                    ..
                } = qself;
                quote! { < #ty as #path > }
            }
            _ => quote! { #path },
        }
    };

    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let qself = match qself {
    //     Some(qself) => Some(quote_as_qself(qself, attr_args)),
    //     _ => qself, // None
    // };
    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let path = quote_as_path(path, attr_args);

    let fields = {
        let mut traversed_fileds = quote! {};
        for field in fields {
            let traversed_field = quote_as_field_value(field, attr_args);
            traversed_fileds = quote! { #traversed_fileds #traversed_field, };
        }
        traversed_fileds
    };
    let rest = rest
        .as_ref()
        .map(|expr| quote_as_expr(&**expr, None, attr_args));

    quote! { #(#attrs)* #qself_and_apth { #fields #dot2_token #rest } }
}
fn quote_as_expr_try(
    expr_try: &ExprTry,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprTry {
        // expr?
        attrs,          //: Vec<Attribute>,
        expr,           //: Box<Expr>,
        question_token, //: Question,
    } = expr_try;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_try };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    quote! { #(#attrs)* #expr #question_token }
}
fn quote_as_expr_try_block(
    expr_try_block: &ExprTryBlock,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprTryBlock {
        // try { ... }
        attrs,     //: Vec<Attribute>,
        try_token, //: Try,
        block,     //: Block,
    } = expr_try_block;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_try_block };
        }
    }
    let block = quote_as_block(block, attr_args);
    quote! { #(#attrs)* #try_token #block }
}
fn quote_as_expr_tuple(
    expr_tuple: &ExprTuple,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprTuple {
        attrs, //: Vec<Attribute>,
        // paren_token, //: Paren,
        elems, //: Punctuated<Expr, Comma>,
        .. // paren_token
    } = expr_tuple;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_tuple };
        }
    }
    let elems = {
        let mut traversed_elems = quote! {};
        for elem in elems {
            let traversed_elem = quote_as_expr(elem, None, attr_args);
            traversed_elems = quote! { #traversed_elems #traversed_elem, }
        }
        traversed_elems
    };
    quote! { #(#attrs)*( #elems ) }
}
fn quote_as_expr_unary(
    expr_unary: &ExprUnary,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprUnary {
        // `!x`, `*x`
        attrs, //: Vec<Attribute>,
        op,    //: UnOp,
        expr,  //: Box<Expr>,
    } = expr_unary;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_unary };
        }
    }
    let expr = quote_as_expr(&**expr, None, attr_args);
    quote! { #(#attrs)* #op #expr }
}
fn quote_as_expr_unsafe(
    expr_unsafe: &ExprUnsafe,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprUnsafe {
        // unsafe { ... }
        attrs,        //: Vec<Attribute>,
        unsafe_token, //: Unsafe,
        block,        //: Block,
    } = expr_unsafe;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_unsafe };
        }
    }
    let block = quote_as_block(block, attr_args);
    quote! { #(#attrs)* #unsafe_token #block }
}
fn quote_as_expr_while(
    expr_while: &ExprWhile,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprWhile {
        attrs,       //: Vec<Attribute>,
        label,       //: Option<Label>,
        while_token, //: While,
        cond,        //: Box<Expr>,
        body,        //: Block,
    } = expr_while;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_while };
        }
    }
    let cond = quote_as_expr(&**cond, None, attr_args);
    let body = quote_as_loop_block(body, attr_args);
    quote! { #(#attrs)* #label #while_token #cond #body }
}
fn quote_as_expr_yield(
    expr_yield: &ExprYield,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ExprYield {
        attrs,       //: Vec<Attribute>,
        yield_token, //: Yield,
        expr,        //: Option<Box<Expr>>,
    } = expr_yield;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #expr_yield };
        }
    }
    let expr = expr
        .as_ref()
        .map(|ref_boxed_expr| quote_as_expr(&**ref_boxed_expr, None, attr_args));
    quote! { #(#attrs)* #yield_token #expr }
}

#[rustfmt::skip]
fn quote_as_expr(expr: &Expr, is_print_func_name: Option<&mut bool>, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match expr {
        Expr::Array     (expr_array) => { quote_as_expr_array(expr_array, attr_args) },
        Expr::Assign    (expr_assign) => { quote_as_expr_assign(expr_assign, attr_args) },
        Expr::Async     (expr_async) => { quote_as_expr_async(expr_async, attr_args) },
        Expr::Await     (expr_await) => { quote_as_expr_await(expr_await, attr_args) },
        Expr::Binary    (expr_binary) => { quote_as_expr_binary(expr_binary, attr_args) },
        Expr::Block     (expr_block) => { quote_as_expr_block(expr_block, attr_args) },
        Expr::Break     (expr_break) => { quote_as_expr_break(expr_break, attr_args) },
        Expr::Call      (expr_call) => { quote_as_expr_call(expr_call, attr_args) },
        Expr::Cast      (expr_cast) => { quote_as_expr_cast(expr_cast, attr_args) },
        Expr::Closure   (expr_closure) => { quote_as_expr_closure(expr_closure, attr_args) },

        // // Likely not applicable for instrumenting the run time functions and 
        // // closures (as opposed to compile time const functions and closures).
        // Expr::Const     (expr_const) => { quote_as_expr_const(expr_const, attr_args) },
        // Expr::Continue  (expr_continue) => { quote_as_expr_continue(expr_continue, attr_args) },

        Expr::Field     (expr_field) => { quote_as_expr_field(expr_field, attr_args) },
        Expr::ForLoop   (expr_for_loop) => { quote_as_expr_for_loop(expr_for_loop, attr_args) },
        Expr::Group     (expr_group) => { quote_as_expr_group(expr_group, attr_args) },
        Expr::If        (expr_if) => { quote_as_expr_if(expr_if, attr_args) },
        Expr::Index     (expr_index) => { quote_as_expr_index(expr_index, attr_args) },
        // // Likely not applicable for instrumenting the run time functions and 
        // // closures (as opposed to compile time const functions and closures).
        // Expr::Infer     (expr_infer) => { quote_as_expr_infer(expr_infer, attr_args) },
        Expr::Let       (expr_let) => { quote_as_expr_let(expr_let, attr_args) },
        // Expr::Lit       (expr_lit) => { quote_as_expr_lit(expr_lit, attr_args) },
        Expr::Loop      (expr_loop) => { quote_as_expr_loop(expr_loop, attr_args) },
        Expr::Macro     (expr_macro) => { quote_as_expr_macro(expr_macro, attr_args) },
        Expr::Match     (expr_match) => { quote_as_expr_match(expr_match, attr_args) },
        Expr::MethodCall(expr_method_call) => { quote_as_expr_method_call(expr_method_call, attr_args) },
        Expr::Paren     (expr_paren) => { quote_as_expr_paren(expr_paren, attr_args) },
        Expr::Path      (expr_path) => { quote_as_expr_path(expr_path, is_print_func_name, attr_args) },
        Expr::Range     (expr_range) => { quote_as_expr_range(expr_range, attr_args) },
        Expr::RawAddr   (expr_raw_addr) => { quote_as_expr_raw_addr(expr_raw_addr, attr_args) },
        Expr::Reference (expr_reference) => { quote_as_expr_reference(expr_reference, attr_args) },
        Expr::Repeat    (expr_repeat) => { quote_as_expr_repeat(expr_repeat, attr_args) },
        Expr::Return    (expr_return) => { quote_as_expr_return(expr_return, attr_args) },
        Expr::Struct    (expr_struct) => { quote_as_expr_struct(expr_struct, attr_args) },
        Expr::Try       (expr_try) => { quote_as_expr_try(expr_try, attr_args) },
        Expr::TryBlock  (expr_try_block) => { quote_as_expr_try_block(expr_try_block, attr_args) },
        Expr::Tuple     (expr_tuple) => { quote_as_expr_tuple(expr_tuple, attr_args) },
        Expr::Unary     (expr_unary) => { quote_as_expr_unary(expr_unary, attr_args) },
        Expr::Unsafe    (expr_unsafe) => { quote_as_expr_unsafe(expr_unsafe, attr_args) },
        Expr::While     (expr_while) => { quote_as_expr_while(expr_while, attr_args) },
        Expr::Yield     (expr_yield) => { quote_as_expr_yield(expr_yield, attr_args) },        

        // Expr::Verbatim  (token_stream) => { quote_as_token_stream(token_stream, attr_args) },
        _other => quote!{ #_other } // Expr::{Macro,Path}
    }
}

fn quote_as_init(init: &LocalInit, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    // `LocalInit` represents `= s.parse()?` in `let x: u64 = s.parse()?` and
    // `= r else { return }` in `let Ok(x) = r else { return }`.
    let LocalInit {
        eq_token, //: Eq,
        expr,     //: Box<Expr>,
        diverge,  //: Option<(Else, Box<Expr>)>,
    } = init;
    let expr = quote_as_expr(expr, None, attr_args);
    let diverge = diverge.as_ref().map(|(else_token, expr)| {
        let expr = quote_as_expr(expr, None, attr_args);
        quote! { #else_token #expr }
    });
    quote! { #eq_token #expr #diverge }
}

fn quote_as_local(local: &Local, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let Local {
        attrs,      //: Vec<Attribute>,
        let_token,  //: Let,
        pat,        //: Pat,
        init,       //: Option<LocalInit>,
        semi_token, //: Semi,
    } = local;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #local };
        }
    }

    let init = init.as_ref().map(|init| quote_as_init(init, attr_args));

    quote! { #(#attrs)* #let_token #pat #init #semi_token }
}

// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_const(item_const: &ItemConst, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemConst { // const MAX: u16 = 65535
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         const_token, //: Const,
//         ident, //: Ident,
//         generics, //: Generics,
//         colon_token, //: Colon,
//         ty, //: Box<Type>,
//         eq_token, //: Eq,
//         expr, //: Box<Expr>,
//         semi_token, //: Semi,
//     } = item_const;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     let generics = quote_as_generics(generics, attr_args);
//     // let ty = quote_as_type(ty, attr_args);
//     let expr = quote_as_expr(expr, attr_args);
//     quote!{ #(#attrs)* #vis #const_token #ident #generics #colon_token #ty #eq_token #expr #semi_token }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_enum(item_enum: &ItemEnum, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemEnum {  // enum Foo<A, B> { A(A), B(B) }
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         enum_token, //: Enum,
//         ident, //: Ident,
//         generics, //: Generics,
//         // brace_token, //: Brace,
//         variants, //: Punctuated<Variant, Comma>,
//         .. // brace_token
//     } = item_enum;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     let generics = quote_as_generics(generics, attr_args);
//     let mut traversed_variants = quote!{};
//     for variant in variants {
//         let traveresed_variant = quote_as_variant(variant, attr_args);
//         traversed_variants = quote!{ #traversed_variants #traveresed_variant }
//     }
//     quote!{ #(#attrs)* #vis #enum_token #ident #generics { #traversed_variants } }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_extern_crate(item_extern_crate: &ItemExternCrate, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemExternCrate {
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         extern_token, //: Extern,
//         crate_token, //: Crate,
//         ident, //: Ident,
//         rename, //: Option<(As, Ident)>,
//         semi_token, //: Semi,
//     } = item_extern_crate;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     quote!{ #(#attrs)* #vis #extern_token #crate_token #ident #rename #semi_token }
// }
trait IsTraverseStopper {
    fn is_traverse_stopper(&self) -> bool;
}
impl IsTraverseStopper for Attribute {
    fn is_traverse_stopper(&self) -> bool {
        let path = match &self.meta {
            Meta::Path(path) => path,
            Meta::List(MetaList { path, .. }) => path,
            Meta::NameValue(MetaNameValue { path, .. }) => path,
        };
        if let Some(last_path_segment) = path.segments.last() {
            let last_path_segment_str = last_path_segment.ident.to_string();
            last_path_segment_str == "loggable" // Will be handled in a separate pass
            // of the preprocessor (otherwise was causing a double instrumentation or smth. like that).
            || last_path_segment_str == "non_loggable"
        } else {
            return false;
        }
    }
}
fn update_param_data_from_pat(
    input_pat: &Pat,
    param_format_str: &mut String,
    param_list: &mut proc_macro2::TokenStream,
) {
    match input_pat {
        // The Rust Reference. ClosureParam.
        // https://doc.rust-lang.org/reference/expressions/closure-expr.html#grammar-ClosureParam
        // https://doc.rust-lang.org/reference/patterns.html#grammar-PatternNoTopAlt
        // https://doc.rust-lang.org/reference/patterns.html#grammar-RangePattern

        // Pat::Const(pat_const) => ?,
        // NOTE: Not found in The Rust Reference (links above) for PatternNoTopAlt.
        // NOTE: Example from ChatGPT looks too rare to fully parse the nested `block`:
        // |const [a, b, c]: [u8; 3]| { println!("{a} {b} {c}"); }
        Pat::Ident(pat_ident) => {
            // x: f32
            let ident = &pat_ident.ident;
            param_format_str.push_str(&format!("{}: {{}}", ident)); // + "x: {}"
            *param_list = quote! { #param_list #ident.maybe_print(), } // + `x.maybe_print(), `
        }
        // Pat::Lit(pat_lit) => ?,  // NOTE: Still questionable: Are literals applicable to params pattern?
        // The Rust Reference mentions/lists it but does not add clarity.
        // ChatGPT states "Not Applicable for params".

        // Pat::Macro(pat_macro) => ?, // NOTE: Out of scope.
        // Pat::Or(pat_or) => ?, // NOTE: Not found in The Rust Reference (for PatternNoTopAlt).
        Pat::Paren(pat_paren) => {
            let PatParen {
                // attrs, //: Vec<Attribute>,
                // paren_token, //: Paren,
                pat, //: Box<Pat>,
                ..
            } = pat_paren;
            param_format_str.push_str(&"(");
            update_param_data_from_pat(pat.as_ref(), param_format_str, param_list);
            param_format_str.push_str(&")");
        }
        // Pat::Path(pat_path) => ?, // NOTE: Example is needed as a param (`path` without `: Type`).
        // Pat::Range(pat_range) => ?, // NOTE: N/A as a param.
        Pat::Reference(pat_reference) => {
            let PatReference {
                // attrs, //: Vec<Attribute>,
                // and_token, //: And, &
                mutability, //: Option<Mut>,
                pat,        //: Box<Pat>,
                ..
            } = pat_reference;
            let mut pat_str = String::with_capacity(32);
            update_param_data_from_pat(pat.as_ref(), &mut pat_str, param_list);

            param_format_str.push_str(&format!("&{} {}", quote! { #mutability }, pat_str)); // + "&mut x: {}"
        }
        // Pat::Rest(pat_rest) => ?, // NOTE: N/A as a param.
        Pat::Slice(pat_slice) => {
            let PatSlice {
                // attrs, //: Vec<Attribute>,
                // bracket_token, //: Bracket,
                elems, //: Punctuated<Pat, Comma>,
                ..
            } = pat_slice;
            param_format_str.push_str(&"[");
            for (idx, elem) in elems.iter().enumerate() {
                if idx != 0 {
                    param_format_str.push_str(&", ");
                }
                update_param_data_from_pat(elem, param_format_str, param_list);
            }
            param_format_str.push_str(&"]");
        }
        Pat::Struct(pat_struct) => {
            // struct MyPoint{ x: i32, y: i32}
            // fn f(MyPoint{x, y: _y}: MyPoint) {}
            // f(MyPoint{ x: 2, y: -4});  // Log: f(MyPoint { x: 2, y: _y: -4 }) {}
            let PatStruct {
                // attrs, // : Vec<Attribute>,
                // qself, // : Option<QSelf>,
                path, // : Path,
                // brace_token, // : Brace,
                fields, // : Punctuated<FieldPat, Comma>,
                        // rest, // : Option<PatRest>,
                ..
            } = pat_struct;
            let mut fields_format_str = String::with_capacity(32);
            for (idx, field) in fields.iter().enumerate() {
                if idx != 0 {
                    fields_format_str.push_str(&", ");
                }
                let FieldPat {
                    // attrs, //: Vec<Attribute>,
                    member,      //: Member,
                    colon_token, //: Option<Colon>,
                    pat,         //: Box<Pat>,
                    ..
                } = field;
                if colon_token.is_some() {
                    let mut member_val_format_str = String::with_capacity(32);
                    let mut member_val_param_list = quote! {};
                    update_param_data_from_pat(
                        pat.as_ref(),
                        &mut member_val_format_str,
                        &mut member_val_param_list,
                    );
                    fields_format_str.push_str(&format!(
                        "{}: {}",
                        quote! {#member},
                        member_val_format_str
                    )); // + "member: MyStruct { <fields> }"
                    *param_list = quote! {
                        #param_list #member_val_param_list // Comma-terminated
                    } // + `field_a.maybe_print(), field_b.maybe_print(), `
                } else {
                    fields_format_str.push_str(&format!("{}: {{}}", quote! {#member})); // + "member: {}"
                    *param_list = quote! { #param_list #member.maybe_print(), } // + `member.maybe_print(), `
                }
            }
            param_format_str.push_str(&format!(
                "{}{{{{{}}}}}", // "MyPoint{{x: {}, y: _y: {}}}"
                remove_spaces(&quote! {#path}.to_string()),
                fields_format_str
            )); // + "MyStruct: { <fileds> }"
        }
        Pat::Tuple(pat_tuple) => {
            let PatTuple {
                // attrs, //: Vec<Attribute>,
                // paren_token, //: Paren,
                elems, //: Punctuated<Pat, Comma>,
                ..
            } = pat_tuple;
            param_format_str.push_str(&"(");
            for (idx, elem) in elems.iter().enumerate() {
                if idx != 0 {
                    param_format_str.push_str(&", ");
                }
                update_param_data_from_pat(elem, param_format_str, param_list);
            }
            param_format_str.push_str(&")");
        }
        Pat::TupleStruct(pat_tuple_struct) => {
            let PatTupleStruct {
                // attrs, //: Vec<Attribute>,
                qself, //: Option<QSelf>,
                path,  //: Path,
                // paren_token, //: Paren,
                elems, //: Punctuated<Pat, Comma>,
                ..
            } = pat_tuple_struct;
            if let Some(qself) = qself {
                let ty = &qself.ty;
                // NOTE: The fragment "<{} as {}>" is questionable.
                param_format_str.push_str(&format!(
                    "<{} as {}>(",
                    quote! { #ty },
                    remove_spaces(&quote! { #path }.to_string())
                ));
            } else {
                param_format_str.push_str(&format!(
                    "{}(",
                    remove_spaces(&quote! { #path }.to_string())
                ));
            }
            for (idx, elem) in elems.iter().enumerate() {
                if idx != 0 {
                    param_format_str.push_str(&", ");
                }
                update_param_data_from_pat(elem, param_format_str, param_list);
            }
            param_format_str.push_str(&")");
        }
        Pat::Type(pat_type) => {
            let PatType {
                // attrs, //: Vec<Attribute>,
                pat, //: Box<Pat>,
                     // colon_token, //: Colon,
                     // ty, //: Box<Type>,
                ..
            } = pat_type;
            update_param_data_from_pat(pat.as_ref(), param_format_str, param_list);
        }
        // Pat::Verbatim(token_stream) // Ignore unclear sequence of tokens among params.
        // Pat::Wild(pat_wild) // Ignore `_` in the pattern.
        _ => {} // Do not print the param values.
    }
}
fn input_vals(inputs: &Punctuated<FnArg, Comma>, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    if inputs.is_empty() {
        quote! { None }
    } else {
        match attr_args.params_logging {
            ParamsLogging::Log => {
                let mut param_format_str = String::new();
                let mut param_list = quote! {};
                for (index, fn_param) in inputs.iter().enumerate() {
                    if index != 0 {
                        param_format_str.push_str(", ");
                    }
                    match fn_param {
                        FnArg::Receiver(_receiver) => {
                            param_format_str.push_str("self: ");
                            if _receiver.reference.is_some() {
                                param_format_str.push_str("&");
                            }
                            if _receiver.mutability.is_some() {
                                param_format_str.push_str("mut ");
                            }
                            param_format_str.push_str("{}");
                            param_list = quote! { #param_list self.maybe_print(), };
                        }
                        FnArg::Typed(pat_type) => {
                            update_param_data_from_pat(&*pat_type.pat, &mut param_format_str, &mut param_list);
                        }
                    }
                }
                quote! { Some(format!(#param_format_str, #param_list)) }
            }
            ParamsLogging::Skip => {
                quote! { Some(String::from("..")) }
            }
        }
    }
}
fn traversed_block_from_sig(
    block: &Block,
    sig: &Signature,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let Signature {
        ident,    //: Ident,
        generics, //: Generics,
        inputs,   //: Punctuated<FnArg, Comma>,
        output,   //: ReturnType,
        ..
    } = sig;
    let inputs = input_vals(inputs, attr_args);

    let mut returns_something = false;
    if let ReturnType::Type(..) = output {
        returns_something = true;
    }

    let block = {
        let func_log_name = {
            if attr_args.prefix.is_empty() {
                quote! { #ident }
            } else {
                let prefix = &attr_args.prefix;
                quote! { #prefix::#ident }
            }
        };

        // Instrument the local functions and closures inside of the function body:
        let attr_args = AttrArgs { 
            prefix: quote! { #func_log_name #generics() },
            params_logging: attr_args.params_logging 
        };
        let block = quote_as_block(block, &attr_args);

        // The proc_macros (the pre-compile) part of the infrastructure for
        // generic parameters substitution with actual generic arguments,
        // i.e. `<T, U>` -> `<char, u8>`.
        let generics_params_iter = generics.type_params();
        let generic_params_is_empty = generics.params.is_empty();

        let func_log_name = remove_spaces(&func_log_name.to_string());

        // Get the multithreading-dependent `logging_is_on()` call token stream:
        let logging_is_on = quote! {
            logger.borrow()
        };
        #[cfg(feature = "singlethreaded")]
        let logging_is_on = quote! {
            #logging_is_on.borrow()
        };
        let logging_is_on = quote! {
            #logging_is_on.logging_is_on()
        };

        // Return the token stream of the instrumented function call:
        quote! {
            {
                use fcl::{CallLogger, MaybePrint};

                let ret_val = fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                    // NOTE: Borrows the parameters. Has to be ahead of the `body`
                    // that moves the parameters to the `body` closure.
                    //
                    // At run time get the string of parameter names and values:
                    let param_val_str = #inputs;

                    // NOTE: The `block` (the function body) will be executed (later)
                    // as a closure (rather than as is)
                    // to handle the `return` in the `block` correctly
                    // (i.e. to catch the return value after the `return` and log that return value).
                    //
                    // Get the function body as a closure:
                    let mut body = move || #block;

                    // If logging is off then do nothing
                    // except executing the body and returning the value:
                    if !#logging_is_on {
                        return body();
                    }
                    // Else (loggign is on):

                    // At run time get the generic function name,
                    // like `f<char,u8>` instead of `f<T,U>`
                    // (at pre-compile (i.e. macro expansion) time the generic arguments
                    // are not known yet):
                    let mut generic_func_name = String::with_capacity(64);
                    generic_func_name.push_str(#func_log_name);
                    if !#generic_params_is_empty {
                        generic_func_name.push_str("<");
                        let generic_arg_names_vec: Vec<&'static str> =
                            vec![#(std::any::type_name::< #generics_params_iter >(),)*];
                        for (idx, generic_arg_name) in generic_arg_names_vec.into_iter().enumerate() {
                            if idx != 0 {
                                generic_func_name.push_str(",");
                            }
                            generic_func_name.push_str(generic_arg_name);
                        }
                        generic_func_name.push_str(">");
                    }

                    // Log the call, like `f(param: 5) {`:
                    let mut callee_logger = fcl::CalleeLogger::new(&generic_func_name, param_val_str);

                    // Execute the function body and catch the return value:
                    let ret_val = body();

                    // Convert the return value to string and assign to the logger:
                    if #returns_something {
                        let ret_val_str = format!("{}", ret_val.maybe_print());
                        callee_logger.set_ret_val(ret_val_str);
                    }

                    // Log the return (and the return value), like `} -> 5 // f().`
                    // in the `CalleeLogger` destructor and return the value to the caller:
                    ret_val
                });
                ret_val
            }
        }
    };

    block
}
fn quote_as_item_fn(
    item_fn: &ItemFn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ItemFn {
        attrs, //: Vec<Attribute>,
        vis,   //: Visibility,
        sig,   //: Signature,
        block, //: Box<Block>,
    } = item_fn;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #item_fn };
        }
    }

    let block = traversed_block_from_sig(block, sig, attr_args);
    quote! { #(#attrs)* #vis #sig #block }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_foreign_mod(item_foreign_mod: &ItemForeignMod, _attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     // let ItemForeignMod {} = item_foreign_mod;
//     quote!{ #item_foreign_mod }
// }
fn quote_as_impl_item_fn(
    impl_item_fn: &ImplItemFn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ImplItemFn {
        attrs,       //: Vec<Attribute>,
        vis,         //: Visibility,
        defaultness, //: Option<Default>,
        sig,         //: Signature,
        block,       //: Block,
    } = impl_item_fn;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #impl_item_fn };
        }
    }
    let block = traversed_block_from_sig(block, sig, attr_args);
    quote! { #(#attrs)* #vis #defaultness #sig #block }
}
fn quote_as_impl_item(
    impl_item: &ImplItem,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    match impl_item {
        ImplItem::Fn(impl_item_fn) => quote_as_impl_item_fn(impl_item_fn, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // ImplItem::Const(impl_item_const) => quote_as_impl_item_const(impl_item_const, attr_args),
        // ImplItem::Type(impl_item_type) => quote_as_impl_item_type(impl_item_type, attr_args),
        // ImplItem::Macro(impl_item_macro) => quote_as_impl_item_macro(impl_item_macro, attr_args),
        // ImplItem::Verbatim(token_stream) => quote_as_token_stream(token_stream, attr_args),
        other => quote! { #other },
    }
}
fn quote_as_item_impl(
    item_impl: &ItemImpl,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ItemImpl {
        attrs, //: Vec<Attribute>,
        defaultness, //: Option<Default>,
        unsafety, //: Option<Unsafe>,
        impl_token, //: Impl,
        generics, //: Generics,
        trait_, //: Option<(Option<Not>, Path, For)>,
        self_ty, //: Box<Type>,
        // brace_token, //: Brace,
        items, //: Vec<ImplItem>,
        .. // brace_token
    } = item_impl;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #item_impl };
        }
    }

    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let generics = quote_as_generics(generics, prefix);

    // Workaround for:
    // the trait bound `(Option<syn::token::Not>, syn::Path, For): quote::ToTokens` is not satisfied
    let trait_ = trait_.as_ref().map(|(opt_not, path, for_token)| {
        quote! { #opt_not #path #for_token }
    });
    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let trait_ = trait_.as_ref().map(|(opt_not, path, for_token)| {
    //     let path = quote_as_path(path);
    //     quote!{ #opt_not #path #for_token };
    // });
    // let self_ty = quote_as_type(&**self_ty, attr_args);

    let items = {
        // Add the impl type to the prefix
        // (to pass such an updated prefix to the nested items):
        let attr_args = AttrArgs { prefix:  
            if attr_args.prefix.is_empty() {
                quote! { #self_ty }
            } else {
                let prefix = &attr_args.prefix;
                quote! { #prefix::#self_ty }
            },
            params_logging: attr_args.params_logging,
        };

        let mut traversed_impl_items = quote! {};
        for impl_item in items {
            let traversed_impl_item = quote_as_impl_item(impl_item, &attr_args);
            traversed_impl_items = quote! { #traversed_impl_items #traversed_impl_item };
        }
        traversed_impl_items
    };
    quote! { #(#attrs)* #defaultness #unsafety #impl_token #generics #trait_ #self_ty { #items } }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_macro(item_macro: &ItemMacro, _attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     // let ItemMacro {} = item_macro;
//     quote!{ #item_macro }
// }
fn quote_as_item_mod(
    item_mod: &ItemMod,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ItemMod {
        attrs,     //: Vec<Attribute>,
        vis,       //: Visibility,
        unsafety,  //: Option<Unsafe>,
        mod_token, //: Mod,
        ident,     //: Ident,
        content,   //: Option<(Brace, Vec<Item>)>,
        semi,      //: Option<Semi>,
    } = item_mod;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #item_mod };
        }
    }

    let attr_args = AttrArgs { prefix:  
        if attr_args.prefix.is_empty() {
            quote! { #ident }
        } else {
            let prefix = &attr_args.prefix;
            quote! { #prefix::#ident }
        },
        params_logging: attr_args.params_logging,
    };

    let content = content.as_ref().map(|(_brace, items)| {
        let mut traversed_items = quote! {};
        for item in items {
            let item = quote_as_item(item, &attr_args);
            traversed_items = quote! { #traversed_items #item };
        }
        quote! { { #traversed_items } }
    });
    quote! { #(#attrs)* #vis #unsafety #mod_token #ident #content #semi }
}
fn quote_as_item_static(
    item_static: &ItemStatic,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ItemStatic {
        attrs,        //: Vec<Attribute>,
        vis,          //: Visibility,
        static_token, //: Static,
        mutability,   //: StaticMutability,
        ident,        //: Ident,
        colon_token,  //: Colon,
        ty,           //: Box<Type>,
        eq_token,     //: Eq,
        expr,         //: Box<Expr>,
        semi_token,   //: Semi,
    } = item_static;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #item_static };
        }
    }

    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let vis = quote_as_vis(vis, attr_args);
    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let ty = quote_as_ty(ty, attr_args);
    let expr = quote_as_expr(expr, None, attr_args);
    quote! { #(#attrs)* #vis #static_token #mutability #ident #colon_token #ty #eq_token #expr #semi_token }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_struct(item_struct: &ItemStruct, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemStruct {
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         struct_token, //: Struct,
//         ident, //: Ident,
//         generics, //: Generics,
//         fields, //: Fields,
//         semi_token, //: Option<Semi>,
//     } = item_struct;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let generics = quote_as_generics(generics, attr_args);
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let fields = {
//     //     let attr_args = AttrArgs { prefix: {
//     //             let prefix = &attr_args.prefix;
//     //             quote!{ #prefix::#ident }
//     //         } 
//     //     };
//     //     let mut traversed_fields = quote!{};
//     //     for field in fields {
//     //         let traversed_field = quote_as_field(field, &attr_args);
//     //         traversed_fields = quote!{ #traversed_fields #traversed_field };
//     //     }
//     // };
//     quote!{ #(#attrs)* #vis #struct_token #ident #generics #fields #semi_token }
// }
fn quote_as_trait_item_const(
    trait_item_const: &TraitItemConst,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let TraitItemConst {
        attrs,       //: Vec<Attribute>,
        const_token, //: Const,
        ident,       //: Ident,
        generics,    //: Generics,
        colon_token, //: Colon,
        ty,          //: Type,
        default,     //: Option<(Eq, Expr)>, // NOTE: Can be (re)assigned in trait impl.
        semi_token,  //: Semi,
    } = trait_item_const;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #trait_item_const };
        }
    }
    let default = default.as_ref().map(|(eq_token, expr)| {
        let expr = quote_as_expr(expr, None, attr_args);
        quote! { #eq_token #expr }
    });
    quote! {  #(#attrs)* #const_token #ident #generics #colon_token #ty #default #semi_token }
}
fn quote_as_trait_item_fn(
    trait_item_fn: &TraitItemFn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let TraitItemFn {
        attrs,      //: Vec<Attribute>,
        sig,        //: Signature,
        default,    //: Option<Block>,
        semi_token, //: Option<Semi>,
    } = trait_item_fn;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #trait_item_fn };
        }
    }
    let default = default
        .as_ref()
        .map(|block| traversed_block_from_sig(block, sig, attr_args));
    quote! { #(#attrs)* #sig #default #semi_token }
}
fn quote_as_trait_item(
    item: &TraitItem,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    match item {
        TraitItem::Const(trait_item_const) => quote_as_trait_item_const(trait_item_const, attr_args),
        TraitItem::Fn(trait_item_fn) => quote_as_trait_item_fn(trait_item_fn, attr_args),

        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // TraitItem::Type(trait_item_type) => quote_as_trait_item_type(trait_item_type, attr_args),
        // TraitItem::Macro(trait_item_macro) => quote_as_trait_item_macro(trait_item_macro, attr_args),

        // TraitItem::Verbatim(token_stream) => quote_as_token_stream(token_stream, attr_args),
        other => quote! { #other },
    }
}
fn quote_as_item_trait(
    item_trait: &ItemTrait,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let ItemTrait {
        attrs, //: Vec<Attribute>,
        vis, //: Visibility,
        unsafety, //: Option<Unsafe>,
        auto_token, //: Option<Auto>,
        // restriction, //: Option<ImplRestriction>,
        trait_token, //: Trait,
        ident, //: Ident,
        generics, //: Generics,
        colon_token, //: Option<Colon>,
        supertraits, //: Punctuated<TypeParamBound, Plus>,
        // brace_token, //: Brace,
        items, //: Vec<TraitItem>,
        .. // restriction, brace_token
    } = item_trait;

    for attr in attrs {
        if attr.is_traverse_stopper() {
            return quote! { #item_trait };
        }
    }

    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let vis = quote_as_vis(vis, attr_args);

    // NOTE: Future: restriction. Unused, but reserved for RFC 3323 restrictions.

    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let generics = quote_as_generics(generics, attr_args);
    // let supertraits = {
    //     let mut traversed_supertraits = quote!{};
    //     for supertrait in supertraits {
    //         let type_param_bound = quote_as_type_param_bound(supertrait, attr_args);
    //         traversed_supertraits = quote!{ #traversed_supertraits #type_param_bound + }
    //     }
    //     traversed_supertraits
    // };
    let items = {
        let attr_args = AttrArgs { prefix:  
            if attr_args.prefix.is_empty() {
                quote! { #ident #generics }
            } else {
                let prefix = &attr_args.prefix;
                quote! { #prefix::#ident #generics }
            },
            params_logging: attr_args.params_logging,
        };
        let mut traversed_items = quote! {};
        for item in items {
            let traversed_item = quote_as_trait_item(item, &attr_args);
            traversed_items = quote! { #traversed_items #traversed_item };
        }
        traversed_items
    };
    quote! { #(#attrs)* #vis #unsafety #auto_token
    #trait_token #ident #generics #colon_token #supertraits { #items } }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_trait_alias(item_trait_alias: &ItemTraitAlias, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemTraitAlias {    // pub trait SharableIterator = Iterator + Sync
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         trait_token, //: Trait,
//         ident, //: Ident,
//         generics, //: Generics,
//         eq_token, //: Eq,
//         bounds, //: Punctuated<TypeParamBound, Plus>,
//         semi_token, //: Semi,
//     } = item_trait_alias;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let generics = quote_as_generics(generics, attr_args);
//     // let bounds = {
//     //     let mut traversed_bounds = quote!{};
//     //     for bound in bounds {
//     //         let type_param_bound = quote_as_type_param_bound(bound, attr_args);
//     //         traversed_bounds = quote!{ #traversed_bounds #type_param_bound + }
//     //     }
//     //     traversed_bounds
//     // };
//     quote!{ #(#attrs)* #vis #trait_token #ident #generics #eq_token #bounds #semi_token }
// }

// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures)
// // since types are a compile time concepts and require const functions
// // executed at compile time.
// fn quote_as_type_array(type_array: &TypeArray, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeArray { // [T; n]
//         // bracket_token, //: Bracket,
//         elem, //: Box<Type>,
//         semi_token, //: Semi,
//         len, //: Expr,
//         .. // bracket_token
//     } = type_array;
//     let elem = quote_as_type(&**elem, attr_args);
//     let len = quote_as_expr(len, attr_args);
//     quote!{ [ #elem #semi_token #len ] }
// }
// fn quote_as_type_bare_fn(type_bare_fn: &TypeBareFn, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeBareFn {
//     } = type_bare_fn;
//     quote!{}
// }
// fn quote_as_type_group(type_group: &TypeGroup, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeGroup {
//     } = type_group;
//     quote!{}
// }
// fn quote_as_type_impl_trait(type_impl_trait: &TypeImplTrait, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeImplTrait {
//     } = type_impl_trait;
//     quote!{}
// }
// fn quote_as_type_infer(type_infer: &TypeInfer, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeInfer {
//     } = type_infer;
//     quote!{}
// }
// fn quote_as_type_macro(type_macro: &TypeMacro, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeMacro {
//     } = type_macro;
//     quote!{}
// }
// fn quote_as_type_never(type_never: &TypeNever, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeNever {
//     } = type_never;
//     quote!{}
// }
// fn quote_as_type_paren(type_paren: &TypeParen, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeParen {
//     } = type_paren;
//     quote!{}
// }
// fn quote_as_type_path(type_path: &TypePath, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypePath {
//     } = type_path;
//     quote!{}
// }
// fn quote_as_type_ptr(type_ptr: &TypePtr, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypePtr {
//     } = type_ptr;
//     quote!{}
// }
// fn quote_as_type_reference(type_reference: &TypeReference, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeReference {
//     } = type_reference;
//     quote!{}
// }
// fn quote_as_type_slice(type_slice: &TypeSlice, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeSlice {
//     } = type_slice;
//     quote!{}
// }
// fn quote_as_type_trait_object(type_trait_object: &TypeTraitObject, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeTraitObject {
//     } = type_trait_object;
//     quote!{}
// }
// fn quote_as_type_tuple(type_tuple: &TypeTuple, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TypeTuple {
//     } = type_tuple;
//     quote!{}
// }
// fn quote_as_token_stream(token_stream: &TokenStream, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let TokenStream {
//     } = token_stream;
//     quote!{}
// }

// // Likely not applicable since types are a compile time concepts and require
// // the const functions (executed at compile time) rather than the run time functions.
// fn quote_as_type(ty: &Type, attr_args: &AttrArgs) -> TokenStream {
//     quote!{ #ty }
//     // match ty {
//     //     Type::Array(type_array) => quote_as_type_array(type_array, attr_args),
//     //     Type::BareFn(type_bare_fn) => quote_as_type_bare_fn(type_bare_fn, attr_args),
//     //     Type::Group(type_group) => quote_as_type_group(type_group, attr_args),
//     //     Type::ImplTrait(type_impl_trait) => quote_as_type_impl_trait(type_impl_trait, attr_args),
//     //     Type::Infer(type_infer) => quote_as_type_infer(type_infer, attr_args),
//     //     Type::Macro(type_macro) => quote_as_type_macro(type_macro, attr_args),
//     //     Type::Never(type_never) => quote_as_type_never(type_never, attr_args),
//     //     Type::Paren(type_paren) => quote_as_type_paren(type_paren, attr_args),
//     //     Type::Path(type_path) => quote_as_type_path(type_path, attr_args),
//     //     Type::Ptr(type_ptr) => quote_as_type_ptr(type_ptr, attr_args),
//     //     Type::Reference(type_reference) => quote_as_type_reference(type_reference, attr_args),
//     //     Type::Slice(type_slice) => quote_as_type_slice(type_slice, attr_args),
//     //     Type::TraitObject(type_trait_object) => quote_as_type_trait_object(type_trait_object, attr_args),
//     //     Type::Tuple(type_tuple) => quote_as_type_tuple(type_tuple, attr_args),
//     //     _other => quote!{ #_other } // Type::Verbatim(token_stream)
//     // }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_type(item_type: &ItemType, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemType {  // type Result<T> = std::result::Result<T, MyError>;
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         type_token, //: Type,
//         ident, //: Ident,
//         generics, //: Generics,
//         eq_token, //: Eq,
//         ty, //: Box<Type>,
//         semi_token, //: Semi,
//     } = item_type;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let generics = quote_as_generics(generics, attr_args);
//     // let ty = quote_as_type(&**ty, attr_args);
//     quote!{ #(#attrs)* #vis #type_token #ident #generics #eq_token #ty #semi_token }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_path(path: &Path, attr_args: &AttrArgs) -> TokenStream {
//     let Path {
//         leading_colon, //: Option<PathSep>,
//         segments, //: Punctuated<PathSegment, PathSep>,
//     } = path;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let segments = {
//     //     let mut traversed_segments = quote!{};
//     //     for segment in segments {
//     //         let segment = quote_as_path_segment(segment, attr_args);
//     //         traversed_segments = quote!{ #traversed_segments #segment:: };
//     //     }
//     // };
//     quote!{ #leading_colon #segments }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_vis_restricted(vis_restricted: &VisRestricted, attr_args: &AttrArgs) -> TokenStream {
//     let VisRestricted { // pub(self) or pub(super) or pub(crate) or pub(in some::module).
//         pub_token, //: Pub,
//         // paren_token, //: Paren,
//         in_token, //: Option<In>,
//         path, //: Box<Path>,
//         .. // paren_token
//     } = vis_restricted;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let path = quote_as_path(&**path, attr_args);
//     quote!{ #pub_token ( #in_token #path ) }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_vis(vis: &Visibility, attr_args: &AttrArgs) -> TokenStream {
//     match vis {
//         Visibility::Restricted(vis_restricted) =>
//             quote_as_vis_restricted(vis_restricted, attr_args),
//         vis_inherited => quote!{ #vis_inherited }, // Public, Inherited
//     }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_generic_param(param: &GenericParam, attr_args: &AttrArgs) -> TokenStream {
//     match param { // `T: Into<String>`, `'a: 'b`, `const LEN: usize`
//         // GenericParam::Type(type_param) => quote_as_type_param(type_param, attr_args),
//         // // Likely not applicable for instrumenting the run time functions and
//         // // closures (as opposed to compile time const functions and closures).
//         // GenericParam::Const(const_param) => quote_as_const_param(const_param, attr_args),
//         _other => quote!{ #_other },    // GenericParam::{Lifetime,Type}
//     }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_generics(generics: &Generics, attr_args: &AttrArgs) -> TokenStream {
//     let Generics {
//         lt_token, //: Option<Lt>,
//         params, //: Punctuated<GenericParam, Comma>,
//         gt_token, //: Option<Gt>,
//         where_clause, //: Option<WhereClause>,
//     } = generics;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let params = {
//     //     let mut traversed_params = quote!{};
//     //     for param in params {
//     //         let generic_param = quote_as_generic_param(param, attr_args);
//     //         traversed_params = quote!{ #traversed_params #generic_param }
//     //     }
//     //     traversed_params
//     // };
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let where_clause = quote_as_where_clause(where_clause, attr_args);
//     quote!{ #lt_token #params #gt_token #where_clause }
// }
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_union(item_union: &ItemUnion, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     let ItemUnion { // union Foo<A, B> { x: A, y: B }
//         attrs, //: Vec<Attribute>,
//         vis, //: Visibility,
//         union_token, //: Union,
//         ident, //: Ident,
//         generics, //: Generics,
//         fields, //: FieldsNamed,
//     } = item_union;
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let vis = quote_as_vis(vis, attr_args);
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let generics = quote_as_generics(generics, attr_args);
//     let attr_args = AttrArgs { prefix: {
//          let prefix = &attr_args.prefix;
//          quote!{ #prefix::#ident }
//     }};
//     // // Likely not applicable for instrumenting the run time functions and
//     // // closures (as opposed to compile time const functions and closures).
//     // let fields = quote_as_fields_named(fields, attr_args);
//     quote!{ #(#attrs)* #vis #union_token #ident #generics #fields }
// }

// fn quote_as_item_use(item_use: &ItemUse, _prefix: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
//     quote!{ #item_use }
// }
// fn quote_as_token_stream(token_stream: &TokenStream, attr_args: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
//     quote!{ #token_stream }
// }

fn quote_as_item(item: &Item, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match item {
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // Item::Const(item_const) => quote_as_item_const(item_const, attr_args),
        // Item::Enum(item_enum) => quote_as_item_enum(item_enum, attr_args),
        // Item::ExternCrate(item_extern_crate) => quote_as_item_extern_crate(item_extern_crate, attr_args),
        Item::Fn(item_fn) => quote_as_item_fn(item_fn, attr_args),
        // Item::ForeignMod(item_foreign_mod) => quote_as_item_foreign_mod(item_foreign_mod, attr_args),
        Item::Impl(item_impl) => quote_as_item_impl(item_impl, attr_args),
        // Item::Macro(item_macro) => quote_as_item_macro(item_macro, attr_args),
        Item::Mod(item_mod) => quote_as_item_mod(item_mod, attr_args),
        Item::Static(item_static) => quote_as_item_static(item_static, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // Item::Struct(item_struct) => quote_as_item_struct(item_struct, attr_args),
        Item::Trait(item_trait) => quote_as_item_trait(item_trait, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // Item::TraitAlias(item_trait_alias) => quote_as_item_trait_alias(item_trait_alias, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // Item::Type(item_type) => quote_as_item_type(item_type, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // Item::Union(item_union) => quote_as_item_union(item_union, attr_args),
        // Item::Use(item_use) => quote_as_item_use(item_use, attr_args),
        // Item::Verbatim(token_stream) => quote_as_token_stream(token_stream, attr_args)
        other => quote! { #other }, // Item::{Const,Enum,Union,Verbatim}
    }
}
fn quote_as_stmt_macro(
    stmt_macro: &StmtMacro,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let StmtMacro {
        attrs,      //: Vec<Attribute>,
        mac,        //: Macro,
        semi_token, //: Option<Semi>,
    } = stmt_macro;

    let mut maybe_flush_invocation = quote! {};
    let mac = quote_as_macro(&mac, &mut maybe_flush_invocation, attr_args);

    let mut ret_val = quote! { #(#attrs)* #mac #semi_token };

    if !maybe_flush_invocation.is_empty() {
        ret_val = quote! {
            {
                #maybe_flush_invocation;
                #ret_val
            }
        }
    }
    ret_val
}
fn quote_as_stmt(stmt: &Stmt, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match stmt {
        Stmt::Local(local) => quote_as_local(local, attr_args),
        Stmt::Item(item) => quote_as_item(item, attr_args),
        Stmt::Expr(expr, opt_semi) => {
            let expr = quote_as_expr(expr, None, attr_args);
            quote! { #expr #opt_semi }
        }
        Stmt::Macro(stmt_macro) => quote_as_stmt_macro(stmt_macro, attr_args),
    }
}
fn quote_as_loop_block(
    block: &Block,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let Block {
        // brace_token, //: Brace,
        stmts, // Vec<Stmt>
        .. //brace_token,
    } = block;

    let stmts = {
        let mut traversed_stmts = quote! {};
        for stmt in stmts {
            let traversed_stmt = quote_as_stmt(stmt, attr_args);
            traversed_stmts = quote! { #traversed_stmts #traversed_stmt }
        }
        traversed_stmts
    };

    // Get the multithreading-dependent `logging_is_on()` call token stream:
    let logging_is_on = quote! {
        logger.borrow()
    };
    #[cfg(feature = "singlethreaded")]
    let logging_is_on = quote! {
        #logging_is_on.borrow()
    };
    let logging_is_on = quote! {
        #logging_is_on.logging_is_on()
    };

    quote! {
        {
            // For now I intentionally leave this reading in every loop iteration
            // so that the user can filter out some iterations from the log
            // by enabling/disabling the logging during the iterations.
            //
            // To accelerate, this reading can be placed in front of the loop
            // (but the check `if logging_is_on` still needs to be in every iteration),
            // such that the reading and the loop are in one extra scope (`{ let logging_is_on = ..; loop }`),
            // and at the end of that scope the `logging_is_on` dies.
            let logging_is_on = fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| { #logging_is_on });

            let _loopbody_logger = if logging_is_on {
                Some(fcl::LoopbodyLogger::new()) // Log the loop body start.
            } else {
                None
            };

            // NOTE: The `loop` can return a value (with the `break <value>` statement),
            // the `for` and `while` cannnot.

            // Execute the loop body
            // (and optionally return a value upon `break <value>` in case of the `loop`):
            //
            // NOTE: The `#stmts` cannot be moved to a closure (as it is done for the body of functions and closures)
            // because `break [<value>]` cannot be executed in a closure (compilation error).
            { // NOTE: This extra scope is to isolate the outer (FCL's) `_loopbody_logger`, `logging_is_on` and possible inner (user's) ones.
                #stmts
            }

            // The loop body end is logged in the destructor of `LoopbodyLogger` instance.
        }
    }
}

fn quote_as_block(block: &Block, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let Block {
        // brace_token, //: Brace,
        stmts, // Vec<Stmt>
        .. //brace_token,
    } = block;

    let stmts = {
        let mut traversed_stmts = quote! {};
        for stmt in stmts {
            let traversed_stmt = quote_as_stmt(stmt, attr_args);
            traversed_stmts = quote! { #traversed_stmts #traversed_stmt }
        }
        traversed_stmts
    };
    quote! { { #stmts } }
}

/// Closure with optional trailing comma
/// (when closure is the last argument of a function):
struct ExprClosureWOptComma {
    closure: ExprClosure,
    _optional_comma: Option<Token![,]>,
}
impl Parse for ExprClosureWOptComma {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(ExprClosureWOptComma {
            closure: input.parse()?,
            _optional_comma: input.parse()?,
        })
    }
}

/// FCL-specific keywords (in particular in the `#[loggable]` attribute).
mod kw {
    // syn::custom_keyword!(name);

    // `#[loggable(prefix = My::Path)]` // <MyStruct as MyPureTrait>
    syn::custom_keyword!(prefix);

    // Skip the parameters logging in the annotated entity and its local entities recursively.
    // 
    // If the function has no prameters then its parameters block is logged as `()`, otherwise `(..)`.
    // ### Examples
    // `#[loggable(skip_params)]`
    syn::custom_keyword!(skip_params);

    // Log the parameters in the annotated entity and its local entities recursively.
    // ### Examples
    // `#[loggable(log_params)]`
    syn::custom_keyword!(log_params);
}

struct FclQSelf {
    // <T as U::V>
    ty: Box<Type>,
    path: Path,
}
impl quote::ToTokens for FclQSelf {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // NOTE: The shortcut below causes an endless recursion.
        // *tokens = quote!{ #self };

        // <T as U::V>
        let FclQSelf {
            // lt_token, // : Token![<],
            ty, // : Box<Type>,
            // as_token, // : Token![as],
            // gt_token, // : Token![>], // TODO: Swap `gt_token` and `path` lines or explain why the order is right.
            path, // : Path,
            ..
        } = self;
        *tokens = quote! { < #ty as #path > };
    }
}
impl Parse for FclQSelf {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        // <T as U::V>
        input.parse::<Token![<]>()?;
        let ty = input.parse()?;
        input.parse::<Token![as]>()?;
        let path = input.parse()?;
        input.parse::<Token![>]>()?;
        Ok(Self { ty, path })
    }
}

enum LogPrefix {
    QSelf(FclQSelf),
    Path(syn::Path),
}
struct QSelfOrPath(Option<LogPrefix>);

impl Parse for QSelfOrPath {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut result = Self(None);
        if input.is_empty() {
            Ok(result)
        } else {
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![<]) {
                result = Self(Some(LogPrefix::QSelf(input.parse()?))); // <T as U::V>
            } else {
                result = Self(Some(LogPrefix::Path(input.parse()?))); // U::V
            }

            Ok(result)
        }
    }
}

#[derive(Copy, Clone)]
enum ParamsLogging {    
    /// Log the parameters (the default). 
    /// ### Examples 
    /// `#[loggable(log_params)]`
    Log,
    /// Skip the parameter logging.
    /// ### Examples 
    /// `#[loggable(skip_params)]`
    Skip,
    // Others, e.g. `Shallow`, // `#[loggable(shallow_params)]` (log params _non-recursively_, i.e. skip (with `..`) the nested structs)
}

struct AttrArgs {
    prefix: proc_macro2::TokenStream,
    /// Tells whether and/or how to log the function or closure parameters. 
    /// ### Examples 
    /// ```ignore
    /// #[loggable(skip_params)] // Skip the parameter logging for function `f()` and its local entities recursively.
    /// fn f(b: bool) {} // Logs: `f(..) {}`.
    /// 
    /// #[loggable(log_params)] // Log the parameters of the functions and closures inside of module `m` recursively.
    /// mod m {
    ///     fn f(b: bool) {}    // Log example: `m::f(b: true) {}`.
    /// }
    /// 
    /// #[loggable] // Has the same effect as `#[loggable(log_params)]`, i.e., the parameters are logged by default.
    /// ```
    params_logging: ParamsLogging
}
impl Parse for AttrArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attr_args = AttrArgs {
            prefix: quote! {},
            params_logging: ParamsLogging::Log,
        };
        loop {
            if input.is_empty() {
                break
            }
            let lookahead = input.lookahead1();
            if lookahead.peek(Token![,]) {   // Skip any sequence of commas before, among, and after the attr args.
                input.parse::<Token![,]>()?;
                continue
            }
            else if lookahead.peek(kw::prefix) {
                input.parse::<kw::prefix>()?;
                input.parse::<Token![=]>()?;
                let optional_prefix = input.parse()?;

                if let QSelfOrPath(Some(q_self_or_path)) = optional_prefix {
                    match q_self_or_path {
                        LogPrefix::QSelf(qself) => attr_args.prefix = quote! { #qself },
                        LogPrefix::Path(path) => attr_args.prefix = quote! { #path },
                    }
                };
            } else if lookahead.peek(kw::skip_params) {
                input.parse::<kw::skip_params>()?;
                attr_args.params_logging = ParamsLogging::Skip;
            } else if lookahead.peek(kw::log_params) {
                input.parse::<kw::log_params>()?;
                attr_args.params_logging = ParamsLogging::Log;
            } else {
                return Err(lookahead.error()); 
                // Reports an error, e.g.,
                // error: expected one of: `,`, `prefix`, `skip_params`, `log_params`
                //    --> fcl\tests\add_call.rs:383:20
            }
        }
        Ok(attr_args)
    }
}
