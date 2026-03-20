use std::str::FromStr;

use crate::{
    AttrArgs, IsTraverseStopper, ParamsLogging, remove_spaces, update_param_data_from_pat,
};
use quote::quote;
use syn::spanned::Spanned;

fn quote_as_expr_array(
    expr_array: &syn::ExprArray,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprArray { // [a, b, c, d]
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
    expr_assign: &syn::ExprAssign,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    // a = compute()
    let syn::ExprAssign {
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

fn quote_as_init(init: &syn::LocalInit, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    // `LocalInit` represents `= s.parse()?` in `let x: u64 = s.parse()?` and
    // `= r else { return }` in `let Ok(x) = r else { return }`.
    let syn::LocalInit {
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

fn quote_as_local(local: &syn::Local, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Local {
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

fn quote_as_stmt_macro(
    stmt_macro: &syn::StmtMacro,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::StmtMacro {
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

fn quote_as_stmt(stmt: &syn::Stmt, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match stmt {
        syn::Stmt::Local(local) => quote_as_local(local, attr_args),
        syn::Stmt::Item(item) => crate::items::quote_as_item(item, attr_args),
        syn::Stmt::Expr(expr, opt_semi) => {
            let expr = quote_as_expr(expr, None, attr_args);
            quote! { #expr #opt_semi }
        }
        syn::Stmt::Macro(stmt_macro) => quote_as_stmt_macro(stmt_macro, attr_args),
    }
}

pub fn quote_as_block(block: &syn::Block, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Block {
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

fn quote_as_expr_async(
    expr_async: &syn::ExprAsync,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprAsync {
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
    expr_await: &syn::ExprAwait,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprAwait {
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
    expr_binary: &syn::ExprBinary,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprBinary {
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
    expr_block: &syn::ExprBlock,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprBlock {
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
    expr_break: &syn::ExprBreak,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprBreak {
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
    let expr = expr
        .as_ref()
        .map(|expr| quote_as_expr(expr, None, attr_args));
    quote! { #(#attrs)* #break_token #label #expr }
}
fn quote_as_expr_call(expr_call: &syn::ExprCall, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::ExprCall {
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
fn quote_as_expr_cast(expr_cast: &syn::ExprCast, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    // foo as f64
    let syn::ExprCast {
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
pub fn quote_as_expr_closure(
    expr_closure: &syn::ExprClosure,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprClosure {
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
    let input_vals = if inputs.is_empty() {
        quote! { None }
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
                quote! { Some(String::from("..")) }
            }
        }
    };

    // Closure name:
    let coords_ts = if attr_args.log_closure_coords {
        let (start_line, start_col) = {
            let proc_macro2::LineColumn { line, column } = or1_token.span().start();
            (line, column + 1)
        };
        let (end_line, end_col) = {
            let proc_macro2::LineColumn { line, column } = body.span().end();
            (line, column)
        };
        let coords_str = format!("{},{}:{},{}", start_line, start_col, end_line, end_col);

        let to_ts_res = proc_macro2::TokenStream::from_str(&coords_str);
        match to_ts_res {
            Ok(ts) => quote! { #ts },
            Err(_lex_err) => quote! { #coords_str },
        }
    } else {
        quote! { .. }
    };
    let mut log_closure_name_ts = quote! { closure{#coords_ts} };
    if !attr_args.prefix.is_empty() {
        let prefix = &attr_args.prefix;
        log_closure_name_ts = quote! { #prefix::#log_closure_name_ts }
    }
    let log_closure_name_str = remove_spaces(&log_closure_name_ts.to_string());
    let attr_args = AttrArgs {
        prefix: log_closure_name_ts,
        ..*attr_args
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
                let mut body = #capture || { #body };

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
    expr_field: &syn::ExprField,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprField {
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

fn quote_as_loop_block(block: &syn::Block, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Block {
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

fn quote_as_expr_for_loop(
    expr_for_loop: &syn::ExprForLoop,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprForLoop {
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
    expr_group: &syn::ExprGroup,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprGroup {
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
fn quote_as_expr_if(expr_if: &syn::ExprIf, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::ExprIf {
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
    expr_index: &syn::ExprIndex,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprIndex {
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
fn quote_as_expr_let(expr_let: &syn::ExprLet, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::ExprLet {
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
fn quote_as_expr_loop(expr_loop: &syn::ExprLoop, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::ExprLoop {
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
        {
            let ret_val = #(#attrs)* #label #loop_token #body ;

            fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger|
                logger.borrow_mut().log_loop_end());

            // TODO:
            // let ret_val_str = format!("{}", ret_val.maybe_print());
            // fcl::call_log_infra::instances::THREAD_LOGGER.with(|thread_logger| {
            //     thread_logger.borrow_mut().set_loop_ret_val(ret_val_str);
            // });
            ret_val
        }
    }
}
pub fn quote_as_macro(
    macro_: &syn::Macro,
    maybe_flush_invocation: &mut proc_macro2::TokenStream,
    _attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::Macro {
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
    expr_macro: &syn::ExprMacro,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprMacro {
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
fn quote_as_arm(arm: &syn::Arm, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Arm {
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
fn quote_as_expr_match(
    expr_match: &syn::ExprMatch,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprMatch {
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
    expr_method_call: &syn::ExprMethodCall,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprMethodCall { // x.foo::<T>(a, b)
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
    expr_paren: &syn::ExprParen,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprParen { // A parenthesized expression: `(a + b)`.
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
    expr_path: &syn::ExprPath,
    is_print_func_name: Option<&mut bool>,
    _attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprPath {
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
    expr_range: &syn::ExprRange,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprRange {
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
    let end = end
        .as_ref()
        .map(|end| quote_as_expr(&**end, None, attr_args));
    quote! { #(#attrs)* #start #limits #end }
}
fn quote_as_expr_raw_addr(
    expr_raw_addr: &syn::ExprRawAddr,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprRawAddr {
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
    expr_reference: &syn::ExprReference,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprReference {
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
    expr_repeat: &syn::ExprRepeat,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprRepeat { // [0u8; N]
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
    expr_return: &syn::ExprReturn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprReturn {
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
fn quote_as_field_value(field: &syn::FieldValue, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::FieldValue {
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
    expr_struct: &syn::ExprStruct,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprStruct { // S { a: 1, b: 1, ..rest }
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
                let syn::QSelf {
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
fn quote_as_expr_try(expr_try: &syn::ExprTry, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::ExprTry {
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
    expr_try_block: &syn::ExprTryBlock,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprTryBlock {
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
    expr_tuple: &syn::ExprTuple,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprTuple {
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
    expr_unary: &syn::ExprUnary,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprUnary {
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
    expr_unsafe: &syn::ExprUnsafe,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprUnsafe {
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
    expr_while: &syn::ExprWhile,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprWhile {
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
    quote! {
        {
            let ret_val = #(#attrs)* #label #while_token #cond #body ;

            fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger|
                logger.borrow_mut().log_loop_end());

            ret_val
        }
    }
}
fn quote_as_expr_yield(
    expr_yield: &syn::ExprYield,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprYield {
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
pub fn quote_as_expr(expr: &syn::Expr, is_print_func_name: Option<&mut bool>, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match expr {
        syn::Expr::Array     (expr_array) => { quote_as_expr_array(expr_array, attr_args) },
        syn::Expr::Assign    (expr_assign) => { quote_as_expr_assign(expr_assign, attr_args) },
        syn::Expr::Async     (expr_async) => { quote_as_expr_async(expr_async, attr_args) },
        syn::Expr::Await     (expr_await) => { quote_as_expr_await(expr_await, attr_args) },
        syn::Expr::Binary    (expr_binary) => { quote_as_expr_binary(expr_binary, attr_args) },
        syn::Expr::Block     (expr_block) => { quote_as_expr_block(expr_block, attr_args) },
        syn::Expr::Break     (expr_break) => { quote_as_expr_break(expr_break, attr_args) },
        syn::Expr::Call      (expr_call) => { quote_as_expr_call(expr_call, attr_args) },
        syn::Expr::Cast      (expr_cast) => { quote_as_expr_cast(expr_cast, attr_args) },
        syn::Expr::Closure   (expr_closure) => { quote_as_expr_closure(expr_closure, attr_args) },

        // // Likely not applicable for instrumenting the run time functions and 
        // // closures (as opposed to compile time const functions and closures).
        // syn::Expr::Const     (expr_const) => { quote_as_expr_const(expr_const, attr_args) },
        // syn::Expr::Continue  (expr_continue) => { quote_as_expr_continue(expr_continue, attr_args) },

        syn::Expr::Field     (expr_field) => { quote_as_expr_field(expr_field, attr_args) },
        syn::Expr::ForLoop   (expr_for_loop) => { quote_as_expr_for_loop(expr_for_loop, attr_args) },
        syn::Expr::Group     (expr_group) => { quote_as_expr_group(expr_group, attr_args) },
        syn::Expr::If        (expr_if) => { quote_as_expr_if(expr_if, attr_args) },
        syn::Expr::Index     (expr_index) => { quote_as_expr_index(expr_index, attr_args) },
        // // Likely not applicable for instrumenting the run time functions and 
        // // closures (as opposed to compile time const functions and closures).
        // syn::Expr::Infer     (expr_infer) => { quote_as_expr_infer(expr_infer, attr_args) },
        syn::Expr::Let       (expr_let) => { quote_as_expr_let(expr_let, attr_args) },
        // syn::Expr::Lit       (expr_lit) => { quote_as_expr_lit(expr_lit, attr_args) },
        syn::Expr::Loop      (expr_loop) => { quote_as_expr_loop(expr_loop, attr_args) },
        syn::Expr::Macro     (expr_macro) => { quote_as_expr_macro(expr_macro, attr_args) },
        syn::Expr::Match     (expr_match) => { quote_as_expr_match(expr_match, attr_args) },
        syn::Expr::MethodCall(expr_method_call) => { quote_as_expr_method_call(expr_method_call, attr_args) },
        syn::Expr::Paren     (expr_paren) => { quote_as_expr_paren(expr_paren, attr_args) },
        syn::Expr::Path      (expr_path) => { quote_as_expr_path(expr_path, is_print_func_name, attr_args) },
        syn::Expr::Range     (expr_range) => { quote_as_expr_range(expr_range, attr_args) },
        syn::Expr::RawAddr   (expr_raw_addr) => { quote_as_expr_raw_addr(expr_raw_addr, attr_args) },
        syn::Expr::Reference (expr_reference) => { quote_as_expr_reference(expr_reference, attr_args) },
        syn::Expr::Repeat    (expr_repeat) => { quote_as_expr_repeat(expr_repeat, attr_args) },
        syn::Expr::Return    (expr_return) => { quote_as_expr_return(expr_return, attr_args) },
        syn::Expr::Struct    (expr_struct) => { quote_as_expr_struct(expr_struct, attr_args) },
        syn::Expr::Try       (expr_try) => { quote_as_expr_try(expr_try, attr_args) },
        syn::Expr::TryBlock  (expr_try_block) => { quote_as_expr_try_block(expr_try_block, attr_args) },
        syn::Expr::Tuple     (expr_tuple) => { quote_as_expr_tuple(expr_tuple, attr_args) },
        syn::Expr::Unary     (expr_unary) => { quote_as_expr_unary(expr_unary, attr_args) },
        syn::Expr::Unsafe    (expr_unsafe) => { quote_as_expr_unsafe(expr_unsafe, attr_args) },
        syn::Expr::While     (expr_while) => { quote_as_expr_while(expr_while, attr_args) },
        syn::Expr::Yield     (expr_yield) => { quote_as_expr_yield(expr_yield, attr_args) },        

        // syn::Expr::Verbatim  (token_stream) => { quote_as_token_stream(token_stream, attr_args) },
        _other => quote!{ #_other } // syn::Expr::{Macro,Path}
    }
}
