#[cfg(feature = "closure_coords_logging")]
use std::str::FromStr;

use crate::{
    common::{
        AttrArgs, remove_spaces, //ParamsLogging, update_param_data_from_pat,
        updated_loggable_attr_args,
    },
    items::quote_as_item,
};
#[cfg(feature = "params_logging")]
use crate::common::{ update_param_data_from_pat, ParamsLogging };


use quote::quote;
#[cfg(feature = "closure_coords_logging")]
use syn::spanned::Spanned;

fn quote_as_expr_array(
    expr_array: &syn::ExprArray,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprArray { // [a, b, c, d]
        attrs, //: Vec<Attribute>,
        // bracket_token, //: Bracket,
        elems, //: Punctuated<Expr, Comma>,
        .. // bracket_token
    } = expr_array;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_array };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_array };
    //     }
    // }
    let elems = {
        if loggable_found {
            quote! { #elems } // TODO: Test.
        } else {
            let mut traversed_elems = quote! {};
            for elem in elems {
                let traversed_elem = quote_as_expr(elem, None, enclosing_item_attr_args);
                traversed_elems = quote! { #traversed_elems #traversed_elem , };
            }
            traversed_elems
        }

        // let mut traversed_elems = quote! {};
        // for elem in elems {
        //     let traversed_elem = quote_as_expr(elem, None, attr_args);
        //     traversed_elems = quote! { #traversed_elems #traversed_elem , };
        // }
        // traversed_elems
    };

    quote! { #(#new_attrs)* [ #elems ] }
    // quote! { #(#attrs)* [ #elems ] }
}

fn quote_as_expr_assign(
    expr_assign: &syn::ExprAssign,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    // a = compute()
    let syn::ExprAssign {
        attrs,    //: Vec<Attribute>,
        left,     //: Box<Expr>,
        eq_token, //: Eq,
        right,    //: Box<Expr>,
    } = expr_assign;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_assign };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_assign };
    //     }
    // }

    let (left, right) = if loggable_found {
        (quote! { #left }, quote! { #right }) // TODO: Test.
    } else {
        (
            quote_as_expr(left, None, enclosing_item_attr_args), // TODO: Test.
            quote_as_expr(right, None, enclosing_item_attr_args), // TODO: Test.
        )
    };
    // let left = quote_as_expr(left, None, enclosing_item_attr_args);
    // let right = quote_as_expr(right, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #left #eq_token #right }
    // quote! { #(#attrs)* #left #eq_token #right }
}

fn quote_as_init(init: &syn::LocalInit, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    // `LocalInit` represents `= s.parse()?` in `let x: u64 = s.parse()?` and
    // `= r else { return }` in `let Ok(x) = r else { return }`.
    //
    // `LocalInit` can also be  like this
    // `
    // #[loggable]
    // fn f() {}
    //
    // ...
    // =        // `LocalInit` starts.
    //   f()    // This expression has limitaions, see https://doc.rust-lang.org/reference/statements.html#grammar-LetStatement
    // [else {
    //   #[loggable]
    //   g() {}
    //   return g();
    // } ]`
    // where nested `#[loggable]`s are handled in the `quote_as_expr()` below,
    // where their `attr_arg`s are combined with those of the enclosing entity.
    // In other words combining the `attr_args` is not applicable here in this fn, is done deeper in the recursion.
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

/// Handles the local `let` binding during the recursive traverse.
///
/// At the moment of writing it is assumed that this fn can only be called as a part of the recursive traverse since
/// ```txt
/// custom attributes cannot be applied to statements
/// see issue #54727 <https://github.com/rust-lang/rust/issues/54727> for more information
/// add `#![feature(proc_macro_hygiene)]` to the crate attributes to enable
/// ```
fn quote_as_local(local: &syn::Local, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Local {
        attrs,      //: Vec<Attribute>,
        let_token,  //: Let,
        pat,        //: Pat,
        init,       //: Option<LocalInit>,
        semi_token, //: Semi,
    } = local;

    // At the moment of writing it is assumed that this fn can only be called as a part of the recursive traverse since
    // ```txt
    // custom attributes cannot be applied to statements
    // see issue #54727 <https://github.com/rust-lang/rust/issues/54727> for more information
    // add `#![feature(proc_macro_hygiene)]` to the crate attributes to enable
    // ```
    // In other words the `let` binding cannot be `#[loggable]`.
    //
    // That is why combining the `#[loggable]` `attr_args` here and those of the enclosing entity is not applicable.
    // If revised then full combining needs to be implemented insted of the outdated code below.
    // Out-of-date:
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #local };
    //     }
    // }

    let init = init.as_ref().map(|init| quote_as_init(init, attr_args));

    quote! { #(#attrs)* #let_token #pat #init #semi_token }
}

/// Handles a macro invocation as a statement.
/// * Prepends the invocation with `maybe_flush()`, if the macro is `[e]print[ln]`;
/// * Surrounds with `{}` the `maybe_flush()` followed by the invocation in case of prepending.
fn quote_as_stmt_macro(
    stmt_macro: &syn::StmtMacro,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::StmtMacro {
        attrs,      //: Vec<Attribute>,
        mac,        //: Macro,
        semi_token, //: Option<Semi>,
    } = stmt_macro;

    // At the moment of writing it is assumed that this fn can only be called as a part of the recursive traverse since
    // ```txt
    // custom attributes cannot be applied to statements
    // see issue #54727 <https://github.com/rust-lang/rust/issues/54727> for more information
    // add `#![feature(proc_macro_hygiene)]` to the crate attributes to enable
    // ```
    // In other words the macro invocation as a statement cannot be `#[loggable]`.
    //
    // So, the `attr_args` combining is not applicable here.
    //
    // TODO: What if the user has `#![feature(proc_macro_hygiene)]` (see a few lines above)
    // and still tries to use `#[loggable]` for statements? Consider in detail what must and what will happen[, document it].
    // Maybe just implement the `attr_args` combining? (It will be dead code by default, but will work as expected with
    // `#![feature(proc_macro_hygiene)]` + {`#[loggable]` on statements}).

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

/// Handles the statements during the recursive traverse.
///
/// At the moment of writing it is assumed that this fn can only be called as a part of the recursive traverse since
/// ```txt
/// custom attributes cannot be applied to statements
/// see issue #54727 <https://github.com/rust-lang/rust/issues/54727> for more information
/// add `#![feature(proc_macro_hygiene)]` to the crate attributes to enable
/// ```
fn quote_as_stmt(stmt: &syn::Stmt, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match stmt {
        syn::Stmt::Local(local) => quote_as_local(local, attr_args),
        syn::Stmt::Item(item) => quote_as_item(item, attr_args, false),
        syn::Stmt::Expr(expr, opt_semi) => {
            let expr = quote_as_expr(expr, None, attr_args);
            quote! { #expr #opt_semi }
        }
        syn::Stmt::Macro(stmt_macro) => quote_as_stmt_macro(stmt_macro, attr_args),
    }
}

pub(crate) fn quote_as_block_statements(
    stmts: &Vec<syn::Stmt>,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let stmts = {
        let mut traversed_stmts = quote! {};
        for stmt in stmts {
            let traversed_stmt = quote_as_stmt(stmt, attr_args);
            traversed_stmts = quote! { #traversed_stmts #traversed_stmt }
        }
        traversed_stmts
    };
    quote! { #stmts }
}

// TODO: pub -> pub(crate) wherever applicable.
pub fn quote_as_block(block: &syn::Block, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Block {
        // brace_token, //: Brace,
        stmts, // Vec<Stmt>
        .. //brace_token,
    } = block;

    let stmts = quote_as_block_statements(stmts, attr_args);
    quote! { { #stmts } }
}

fn quote_as_expr_async(
    expr_async: &syn::ExprAsync,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprAsync {
        // async { ... }
        attrs,       //: Vec<Attribute>,
        async_token, //: Async,
        capture,     //: Option<Move>,
        block,       //: Block,
    } = expr_async;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_async };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_async };
    //     }
    // }

    let block = if loggable_found {
        quote! { #block } // TODO: Test.
    } else {
        quote_as_block(block, enclosing_item_attr_args) // TODO: Test.
    };
    // let block = quote_as_block(block, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #async_token #capture #block } // TODO: Test.
    // quote! { #(#attrs)* #async_token #capture #block }
}

fn quote_as_expr_await(
    expr_await: &syn::ExprAwait,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprAwait {
        // fut.await
        attrs,       //: Vec<Attribute>,
        base,        //: Box<Expr>,
        dot_token,   //: Dot,
        await_token, //: Await,
    } = expr_await;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_await };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_await };
    //     }
    // }

    let base = if loggable_found {
        quote! { #base } // TODO: Test.
    } else {
        quote_as_expr(base, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let base = quote_as_expr(base, None, attr_args);

    quote! { #(#new_attrs)* #base #dot_token #await_token } // TODO: Test.
    // quote! { #(#attrs)* #base #dot_token #await_token }
}

/// Handles the binary operator expressions (like `a + b`, `a += b`).
fn quote_as_expr_binary(
    expr_binary: &syn::ExprBinary,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprBinary {
        // `a + b`, `a += b`
        attrs, //: Vec<Attribute>,
        left,  //: Box<Expr>,
        op,    //: BinOp,
        right, //: Box<Expr>,
    } = expr_binary;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_binary };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_binary };
    //     }
    // }

    let (left, right) = if loggable_found {
        (quote! { #left }, quote! { #right }) // TODO: Test.
    } else {
        (
            quote_as_expr(left, None, enclosing_item_attr_args), // TODO: Test.
            quote_as_expr(right, None, enclosing_item_attr_args), // TODO: Test.
        )
    };
    // let left = quote_as_expr(left, None, enclosing_item_attr_args);
    // let right = quote_as_expr(right, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #left #op #right }
    // quote! { #(#attrs)* #left #op #right }
}

fn quote_as_expr_block(
    expr_block: &syn::ExprBlock,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprBlock {
        attrs, //: Vec<Attribute>,
        label, //: Option<Label>,
        block, //: Block,
    } = expr_block;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_block };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_block };
    //     }
    // }

    let block = if loggable_found {
        quote! { #block } // TODO: Test.
    } else {
        quote_as_block(block, enclosing_item_attr_args) // TODO: Test.
    };
    // let block = quote_as_block(block, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #label #block }
    // quote! { #(#attrs)* #label #block }
}

/// Handles a `break ['label] [<break return value>]`.
fn quote_as_expr_break(
    expr_break: &syn::ExprBreak,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprBreak {
        attrs,       //: Vec<Attribute>,
        break_token, //: Break,
        label,       //: Option<Lifetime>,
        expr,        //: Option<Box<Expr>>,
    } = expr_break;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_break };
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_break };
    //     }
    // }

    let expr = expr.as_ref().map(|expr| {
        if loggable_found {
            quote! { #expr }
        } else {
            quote_as_expr(expr, None, enclosing_item_attr_args)
        }
        // quote_as_expr(expr, None, enclosing_item_attr_args)
    });

    quote! { #(#new_attrs)* #break_token #label #expr }
    // quote! { #(#attrs)* #break_token #label #expr }
}

/// Handles a function call expression: `invoke(a, b)`, `<expr>(<expr>, <expr>)`.
///
/// If the call is a standard output function, e.g. `"_[e]print"` (see details in `quote_as_expr_path()`),
/// then prepends it with `maybe_flush()`, i.e. converts it to the following
/// (to flush the FCL cache before the user's standard ouput)
/// ```ignore
/// {
///     ... maybe_flush();
///     <this call>
/// }
/// ```
fn quote_as_expr_call(
    expr_call: &syn::ExprCall,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprCall {
        attrs, //: Vec<Attribute>,
        func, //: Box<Expr>,
        // paren_token, //: Paren,
        args, //: Punctuated<Expr, Comma>,
        .. // paren_token
    } = expr_call;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_call }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_call };
    //     }
    // }

    let mut is_print_func_name = false;

    let (func, args) = if loggable_found {
        (quote! { #func }, quote! { #args }) // TODO: Test.
    } else {
        (
            quote_as_expr(
                func,
                Some(&mut is_print_func_name),
                enclosing_item_attr_args,
            ), // TODO: Test.
            {
                let mut traversed_args = quote! {};
                for arg in args {
                    let traversed_arg = quote_as_expr(arg, None, enclosing_item_attr_args);
                    traversed_args = quote! { #traversed_args #traversed_arg, }
                }
                traversed_args // TODO: Test.
            }, // quote_as_expr(args, None, enclosing_item_attr_args),
        )
    };
    // let func = quote_as_expr(
    //     func,
    //     Some(&mut is_print_func_name),
    //     enclosing_item_attr_args,
    // );
    // let args = {
    //     let mut traversed_args = quote! {};
    //     for arg in args {
    //         let traversed_arg = quote_as_expr(arg, None, enclosing_item_attr_args);
    //         traversed_args = quote! { #traversed_args #traversed_arg, }
    //     }
    //     traversed_args
    // };

    let mut ret_val = quote! { #(#new_attrs)* #func ( #args ) };

    if is_print_func_name {
        let (extra_use, extra_borrow) = if cfg!(feature = "single_threaded") {
            (
                quote! { use std::borrow::BorrowMut },
                quote! { .borrow_mut() },
            )
        } else {
            (quote! {}, quote! {})
        };

        let thread_logger_access = quote! {
            { // Limit the sope to avoid the `use std::borrow::BorrowMut` below causing warnings or conflicts
              // (the scope is applicable to `cfg!(feature = "single_threaded")` only, but doesn't harm otherwise).

                #extra_use; // use std::borrow::BorrowMut;
                fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                    logger.borrow_mut() #extra_borrow .maybe_flush();
                })
            }
        };
        // #[cfg(feature = "single_threaded")]
        // let thread_logger_access = quote! {
        //     { // Limit the sope to avoid the `use std::borrow::BorrowMut` below causing warnings or conflicts.
        //         use std::borrow::BorrowMut;
        //         fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
        //             logger.borrow_mut().borrow_mut().maybe_flush();
        //         })
        //     }
        // };
        // #[cfg(feature = "multithreaded")]
        // let thread_logger_access = quote! {
        //     fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
        //         logger.borrow_mut().maybe_flush();
        //     })
        // };
        ret_val = quote! {
            {
                #thread_logger_access;
                #ret_val
            }
        }
    };
    ret_val
}

/// Handles a cast expression: `foo as f64`.
fn quote_as_expr_cast(
    expr_cast: &syn::ExprCast,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    // foo as f64
    let syn::ExprCast {
        attrs,    //: Vec<Attribute>,
        expr,     //: Box<Expr>,
        as_token, //: As,
        ty,       //: Box<Type>,
    } = expr_cast;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_cast }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_cast };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #expr #as_token #ty }
    // quote! { #(#attrs)* #expr #as_token #ty }
}

/// Handles a closure expression: `|a, b| a + b`.
pub fn quote_as_expr_closure(
    expr_closure: &syn::ExprClosure,
    enclosing_item_attr_args: &AttrArgs,
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

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_closure }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_closure };
    //     }
    // }

    #[cfg(feature = "params_logging")]
    fn closure_input_vals(
        inputs: &syn::punctuated::Punctuated<syn::Pat, syn::token::Comma>, 
        enclosing_item_attr_args: &AttrArgs
    ) -> proc_macro2::TokenStream {
        if inputs.is_empty() {
            quote! { None }
        } else {
            match enclosing_item_attr_args.params_logging {
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
        }
    }

    #[cfg(feature = "params_logging")]
    let (get_inputs_str_code, pass_inputs_str_code) = (
        {
            let input_vals = closure_input_vals(inputs, enclosing_item_attr_args);
            quote!{ let param_val_str = #input_vals }
        },
        quote!{ param_val_str }
    );
    #[cfg(not(feature = "params_logging"))]
    let (get_inputs_str_code, pass_inputs_str_code) = (
        quote!{}, quote!{}
    );

    // // Get the token stream of {{param names and values} optional string}:
    // #[cfg(feature = "params_logging")]
    // let input_vals = closure_input_vals(inputs, enclosing_item_attr_args)
    // /*if inputs.is_empty() {
    //     quote! { None }
    // } else {
    //     match enclosing_item_attr_args.params_logging {
    //         ParamsLogging::Log => {
    //             let mut param_format_str = String::new();
    //             let mut param_list = quote! {};
    //             for (idx, input_pat) in inputs.iter().enumerate() {
    //                 if idx != 0 {
    //                     param_format_str.push_str(", ");
    //                 }
    //                 update_param_data_from_pat(input_pat, &mut param_format_str, &mut param_list);
    //             }
    //             quote! { Some(format!(#param_format_str, #param_list)) }
    //         }
    //         ParamsLogging::Skip => {
    //             quote! { Some(String::from("..")) }
    //         }
    //     }
    // }*/;
    // #[cfg(not(feature = "params_logging"))]
    // let input_vals = quote! {};

    // Closure coordinates:
    #[cfg(feature = "closure_coords_logging")]
    let coords_ts = if enclosing_item_attr_args.log_closure_coords {
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
            Err(_lex_err) => quote! { #coords_str }, // TODO: Must never get here? Comment in detail.
        }
    } else {
        quote! { .. }
    };
    #[cfg(not(feature = "closure_coords_logging"))]
    let coords_ts = quote! {};

    // Closure name:
    let mut log_closure_name_ts = quote! { closure{#coords_ts} };
    if !enclosing_item_attr_args.prefix.is_empty() {
        let prefix = &enclosing_item_attr_args.prefix;
        log_closure_name_ts = quote! { #prefix::#log_closure_name_ts }
    }
    let log_closure_name_str = remove_spaces(&log_closure_name_ts.to_string());
    let attr_args = AttrArgs {
        prefix: log_closure_name_ts,
        ..*enclosing_item_attr_args
    };

    // Optionally instrument the closure body:
    let body = if loggable_found {
        quote! { #body } // TODO: Test.
    } else {
        quote_as_expr(&**body, None, &attr_args) // TODO: Test.
    };
    // let body = { quote_as_expr(&**body, None, &attr_args) };

    let extra_borrow = if cfg!(feature = "single_threaded") {
        quote! { .borrow_mut() }
    } else {
        quote! {}
    };
    // // `logging_is_on()`:
    // let logging_is_on = quote! {
    //     logger.borrow()
    // };
    // #[cfg(feature = "single_threaded")]
    // let logging_is_on = quote! {
    //     #logging_is_on.borrow()
    // };
    // let logging_is_on = quote! {
    //     #logging_is_on.logging_is_on()
    // };

    #[cfg(feature = "ret_val_logging")]
    let ret_val_logging_code = quote! {
        use fcl::common::{MaybePrint};
        // Uncondititonally tell the `callee_logger` what closure returns,
        // since if the closure's return type is not specified explicitly
        // then the return type is determined with the type inference
        // which is not available now at pre-compile (preprocessing) time.
        // In other words, at pre-compile time we don't know for sure
        // if {the closure return type is the unit type `()` and the return value logging can be skipped}.
        let ret_val_str = format!("{}", ret_val.maybe_print());
        callee_logger.set_ret_val(ret_val_str);
    };
    #[cfg(not(feature = "ret_val_logging"))]
    let ret_val_logging_code = quote! {};

    // Return the token stream of the instrumented closure:
    // TODO: Test.
    quote! {
        #(#new_attrs)*
        // #(#attrs)*
        #lifetimes #constness #movability #asyncness #capture
        #or1_token #inputs #or2_token #output
        {
            use fcl::common::{CallLogger/*, MaybePrint*/};

            let ret_val = fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| { // NOTE: The `logger` is used in `logging_is_on`.
                // // NOTE: Borrows the params, has to be in front of the `body`
                // // that moves the params to the `body` closure.
                // //
                // // At run time get the parameter names and values string:
                #get_inputs_str_code;
                // let param_val_str = #input_vals;

                // Get the body as a closure (to be executed later):
                let mut body = #capture || { #body };

                // If logging is off then do nothing
                // except executing the body and returning the value:
                if ! logger.borrow() #extra_borrow .logging_is_on() {
                // if ! #logging_is_on {
                    return body();
                }
                // Else (logging is on):

                // Log the call, like `f()::closure{3,7:5:11}(param: true) {`:
                let mut callee_logger = fcl::common::CalleeLogger::new(
                    #log_closure_name_str,
                    #pass_inputs_str_code   // NOTE: Comma `,` is not allowed here? TODO: Find out for sure.
                    // #input_vals     // NOTE: Comma `,` is not allowed here.
                    // param_val_str
                );

                // Execute the body and catch the return value:
                let ret_val = body();

                #ret_val_logging_code;
                // // Uncondititonally tell the `callee_logger` what closure returns,
                // // since if the closure's return type is not specified explicitly
                // // then the return type is determined with the type inference
                // // which is not available now at pre-compile (preprocessing) time.
                // // In other words, at pre-compile time we don't know for sure
                // // if {the closure return type is the unit type `()` and the return value logging can be skipped}.
                // let ret_val_str = format!("{}", ret_val.maybe_print());
                // callee_logger.set_ret_val(ret_val_str);

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
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprField {
        attrs,     //: Vec<Attribute>,
        base,      //: Box<Expr>,
        dot_token, //: Dot,
        member,    //: Member,
    } = expr_field;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_field }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_field };
    //     }
    // }

    let base = if loggable_found {
        quote! { #base } // TODO: Test.
    } else {
        quote_as_expr(&**base, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let base = quote_as_expr(&**base, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #base #dot_token #member }
    // quote! { #(#attrs)* #base #dot_token #member }
}

/// Handles the block of `loop`, `for`, and `while` loops.
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

    let extra_borrow = if cfg!(feature = "single_threaded") {
        quote! { .borrow_mut() }
    } else {
        quote! {}
    };
    // // Get the multithreading-dependent `logging_is_on()` call token stream:
    // let logging_is_on = quote! {
    //     logger.borrow()
    // };
    // #[cfg(feature = "single_threaded")]
    // let logging_is_on = quote! {
    //     #logging_is_on.borrow()
    // };
    // let logging_is_on = quote! {
    //     #logging_is_on.logging_is_on()
    // };

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
            let logging_is_on = fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                logger.borrow() #extra_borrow .logging_is_on()
                // #logging_is_on
            });

            let _loopbody_logger = if logging_is_on {
                Some(fcl::common::LoopbodyLogger::new()) // Log the loop body start.
            } else {
                None
            };

            // NOTE: The `loop` can return a value (with the `break <value>` statement),
            // the `for` and `while` cannnot.

            // Execute the loop body
            // (and optionally return a value upon `break <value>` in case of the `loop`):
            //
            // NOTE: The `#stmts` cannot be moved to a closure (similar to the body of functions and closures)
            // because `break [<value>]` cannot be executed in a closure (compilation error).
            { // NOTE: This extra scope is to isolate the outer (FCL's) `_loopbody_logger`, `logging_is_on` and possible inner (user's) ones.
                #stmts
            }

            // The loop body end is logged in the destructor of `LoopbodyLogger` instance `_loopbody_logger`.
        }
    }
}

/// Handles a `for` loop: `for pat in expr { ... }`.
fn quote_as_expr_for_loop(
    expr_for_loop: &syn::ExprForLoop,
    enclosing_item_attr_args: &AttrArgs,
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

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_for_loop }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_for_loop };
    //     }
    // }

    let (expr, body) = if loggable_found {
        (quote! { #expr }, quote! { #body }) // TODO: Test.
    } else {
        (
            quote_as_expr(&**expr, None, enclosing_item_attr_args), // TODO: Test.
            quote_as_loop_block(body, enclosing_item_attr_args),    // TODO: Test.
        )
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);
    // let body = quote_as_loop_block(body, enclosing_item_attr_args);

    let extra_borrow = if cfg!(feature = "single_threaded") {
        quote! { .borrow_mut() } // TODO: Test with `#[cfg(feature = "single_threaded")]`, either update or document.
    } else {
        quote! {}
    };

    quote! {
        {
            let ret_val = { // At the moment of writing the unit value `()`
                // is the only known possible value returnable by `for` loop.
                #(#new_attrs)* #label #for_token #pat #in_token #expr #body
            };

            fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger|
                logger.borrow_mut() #extra_borrow .log_loop_end());

            ret_val
        }
    }
}

/// Handles the expression contained within invisible delimiters,
/// important for faithfully representing the precedence of expressions.
fn quote_as_expr_group(
    expr_group: &syn::ExprGroup,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprGroup {
        attrs, //: Vec<Attribute>,
        // group_token, //: Group,  // None-delimited group. Has nothing but the span and is not `quote`able.
        expr, //: Box<Expr>,
        .. // group_token
    } = expr_group;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_group }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_group };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    // NOTE:
    // Intention: `quote! { { #(#attrs)* #group_token #expr } }`
    // Issue: the trait bound `syn::token::Group: quote::ToTokens` is not satisfied
    // Workaround:
    quote! { { #(#new_attrs)* #expr } }
    // quote! { { #(#attrs)* #expr } }
}

/// Handles an `if` expression with an optional `else` block: `if expr { ... } else { ... }`.
fn quote_as_expr_if(
    expr_if: &syn::ExprIf,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprIf {
        attrs,       //: Vec<Attribute>,
        if_token,    //: If,
        cond,        //: Box<Expr>,
        then_branch, //: Block,
        else_branch, //: Option<(Else, Box<Expr>)>,
    } = expr_if;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_if }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_if };
    //     }
    // }

    let (cond, then_branch, else_branch) = if loggable_found {
        (
            quote! { #cond },
            quote! { #then_branch },
            // NOTE: Workaround for `quote! { #else_branch } `:
            else_branch.as_ref().map(|(else_token, expr)| {
                quote! { #else_token #expr }
            }),
        ) // TODO: Test.
    } else {
        let cond = quote_as_expr(&**cond, None, enclosing_item_attr_args); // TODO: Test.
        let then_branch = quote_as_block(then_branch, enclosing_item_attr_args); // TODO: Test.
        let else_branch = else_branch.as_ref().map(|(else_token, expr)| {
            let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);
            quote! { #else_token #expr }
        }); // TODO: Test.
        (cond, then_branch, else_branch)
        // quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };

    // let cond = quote_as_expr(&**cond, None, enclosing_item_attr_args);
    // let then_branch = quote_as_block(then_branch, enclosing_item_attr_args);
    // let else_branch = else_branch.as_ref().map(|(else_token, expr)| {
    //     let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);
    //     quote! { #else_token #expr }
    // });

    quote! { #(#new_attrs)* #if_token #cond #then_branch #else_branch }
    // quote! { #(#attrs)* #if_token #cond #then_branch #else_branch }
}

/// Handles a square bracketed indexing expression: `vect[2]`.
fn quote_as_expr_index(
    expr_index: &syn::ExprIndex,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprIndex {
        attrs, //: Vec<Attribute>,
        expr, //: Box<Expr>,
        // bracket_token, //: Bracket,
        index, //: Box<Expr>,
        .. // bracket_token
    } = expr_index;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_index }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_index };
    //     }
    // }

    let (expr, index) = if loggable_found {
        (quote! { #expr }, quote! { #index }) // TODO: Test.
    } else {
        (
            quote_as_expr(&**expr, None, enclosing_item_attr_args), // TODO: Test.
            quote_as_expr(&**index, None, enclosing_item_attr_args), // TODO: Test.
        )
    };
    // let expr  = quote_as_expr(&**expr, None, enclosing_item_attr_args);
    // let index = quote_as_expr(&**index, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #expr [ #index ] }
    // quote! { #(#attrs)* #expr [ #index ] }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_expr_infer(expr_infer: &ExprInfer, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     quote!{ #expr_infer }
// }

/// Handles a `let` guard: `let Some(x) = opt`.
fn quote_as_expr_let(
    expr_let: &syn::ExprLet,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprLet {
        attrs,     //: Vec<Attribute>,
        let_token, //: Let,
        pat,       //: Box<Pat>,
        eq_token,  //: Eq,
        expr,      //: Box<Expr>,
    } = expr_let;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_let }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_let };
    //     }
    // }

    // // Likely not applicable for instrumenting the run time functions and
    // // closures (as opposed to compile time const functions and closures).
    // let pat = quote_as_pat(&**pat, attr_args);

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #let_token #pat #eq_token #expr }
    // quote! { #(#attrs)* #let_token #pat #eq_token #expr }
}
// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_expr_lit(expr_lit: &ExprLit, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     quote!{ #expr_lit }
// }

/// Handles a conditionless `loop`: `loop { ... }`.
fn quote_as_expr_loop(
    expr_loop: &syn::ExprLoop,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprLoop {
        attrs,      //: Vec<Attribute>,
        label,      //: Option<Label>,
        loop_token, //: Loop,
        body,       //: Block,
    } = expr_loop;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_loop }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_loop };
    //     }
    // }

    let body = if loggable_found {
        quote! { #body } // TODO: Test.
    } else {
        quote_as_loop_block(body, enclosing_item_attr_args) // TODO: Test.
    };
    // let body = quote_as_loop_block(body, enclosing_item_attr_args);

    let extra_borrow = if cfg!(feature = "single_threaded") {
        quote! { .borrow_mut() } // TODO: Test with `#[cfg(feature = "single_threaded")]`, either update or document.
    } else {
        quote! {}
    };
    quote! {
        // Ret val for `loop` has been deprioritized since it requires extra
        // refactoring for the case of a (removed) loopbody with no nested calls.
        {
            let ret_val = #(#new_attrs)* #label #loop_token #body;

            // TODO:
            // #[cfg(feature = "ret_val_logging")]
            // {
            //     let ret_val_str = format!("{}", ret_val.maybe_print());
            //     fcl::call_log_infra::instances::THREAD_LOGGER.with(|thread_logger| {
            //         thread_logger.borrow_mut().set_loop_ret_val(ret_val_str);
            //     });
            // }

            fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger|
                logger.borrow_mut() #extra_borrow .log_loop_end());

            ret_val
        }
    }
}

/// * Assigns `quote { logger.borrow_mut()[.borrow_mut()].maybe_flush(); }` to the parameter `maybe_flush_invocation`,
///   if the macro name is `"[e]print[ln]"`.
/// * Quotes (`quote{ ... }`) the macro as is. Ignores the `_attr_args` parameter.
pub fn quote_as_macro(
    macro_: &syn::Macro,
    maybe_flush_invocation: &mut proc_macro2::TokenStream, // TODO: Consider `&mut Option<proc_macro2::TokenStream>`.
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
        // TODO: Consider single `if`.
        if &macro_name.ident.to_string() == &"println"
            || &macro_name.ident.to_string() == &"print"
            || &macro_name.ident.to_string() == &"eprintln"
            || &macro_name.ident.to_string() == &"eprint"
        {
            // TODO: Assert the optional last-but-one path segment is "std"
            // (and no more path segments,
            // or their simplified/canonical form ends up in `std`, e.g., `std::something_unrelated::..` is equivalent to `std`,
            // if `..` is supported in the paths).

            let extra_borrow = if cfg!(feature = "single_threaded") {
                quote! { .borrow_mut() }
            } else {
                quote! {}
            };

            let thread_logger_access = quote! {
                fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
                    logger.borrow_mut() #extra_borrow .maybe_flush();
                })
            };
            // #[cfg(feature = "single_threaded")]
            // let thread_logger_access = quote! {
            //     fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            //         logger.borrow_mut().borrow_mut().maybe_flush();
            //     })
            // };
            // #[cfg(feature = "multithreaded")]
            // let thread_logger_access = quote! {
            //     fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            //         logger.borrow_mut().maybe_flush();
            //     })
            // };

            *maybe_flush_invocation = thread_logger_access;
            // *maybe_flush_invocation = quote! {  // TODO: Consider `*maybe_flush_invocation = thread_logger_access`.
            //     #thread_logger_access;
            // }
        }
    }
    quote! { #macro_ } // TODO: Consider returning `()`.
}

/// Handles a macro invocation expression: `format!("{}", q)`.
///
/// If the macro name is `"[e]print[ln]"`, then
/// * prepends it with `..maybe_flush();`,
/// * surrounds both with `{}`.
///
/// ### Example
/// The following `#[loggable]` macro invocation
/// ```ignore
/// #[fcl_proc_macros::loggable]    // The line of interest.
/// fn f() {
///     println!("OK")              // The line of interest.
/// }
/// ```
/// expands to
/// ```ignore
/// fn f() {
///     { // The block of interest.
///         fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
///             logger.borrow_mut().maybe_flush();
///         });
///         println!("OK")
///     }
/// }
/// ```
// TODO: Consider in detail the `#[loggable]` (declarative) macro invocation _expressions_.
// * Ideally implement unified with
//   * "fcl_doc/mdBook.md" / "The Declarative (`macro_rules`) Macros That Are `#[loggable]`",
//   * see in action in "fcl\tests\proc_macros\proc_macro_args\trait_macro.rs".
// * Document it as a continuation of that chapter, explaining that
//   * "attributes on expressions are experimental", that's why the macro invocation _expressions_
//     cannot be `#[loggable]` by default at the moment of writing,
//   * but later the use of `#[loggable]` on macro invocation expressions can stop being experimental
//     (or if really needed now, then `#![feature(stmt_expr_attributes)]` can help),
//     see details in a NOTE in front of `quote_as_expr()`.
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

    if maybe_flush_invocation.is_empty() {
        quote! { #expr_macro }
    } else {
        quote! {
            {
                #maybe_flush_invocation;
                #(#attrs)* #mac
            }
        }
    }
}
/// Handles one arm of a `match` expression: `0..=10 => { return true; }`.
///
/// As in:
/// ```
/// fn f(n: u8) -> bool {
///     match n {
///         0..=10 => {
///             return true;
///         }
///         _ => { todo!() }
///     }
/// }
/// ```
fn quote_as_arm(arm: &syn::Arm, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::Arm {
        attrs,           //: Vec<Attribute>,
        pat,             //: Pat,
        guard,           //: Option<(If, Box<Expr>)>,
        fat_arrow_token, //: FatArrow,
        body,            //: Box<Expr>,
        comma,           //: Option<Comma>,
    } = arm;

    // NOTE: The `match` arms cannot be `#[loggable]` and `#[non_loggable]`:
    // ```txt
    // Compiler Error:
    //     expected non-macro attribute, found attribute macro `loggable`
    //     not a non-macro attribute
    // ```
    // Thus, combining the `attr_args` is not applicable.
    //
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #arm };
    //     }
    // }

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

/// Handles a `match` expression: `match n { Some(n) => {}, None => {} }`.
fn quote_as_expr_match(
    expr_match: &syn::ExprMatch,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprMatch {
        attrs, //: Vec<Attribute>,
        match_token, //: Match,
        expr, //: Box<Expr>,
        // brace_token, //: Brace,
        arms, //: Vec<Arm>,
        .. // brace_token
    } = expr_match;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_match }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_match };
    //     }
    // }

    let (expr, arms) = if loggable_found {
        (quote! { #expr }, quote! { #(#arms)* }) // TODO: Test.
    } else {
        let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args); // TODO: Test.
        let mut traveresed_arms = quote! {}; // TODO: Test.
        for arm in arms {
            let traversed_arm = quote_as_arm(arm, enclosing_item_attr_args);
            traveresed_arms = quote! { #traveresed_arms #traversed_arm }
        }
        (expr, traveresed_arms)
    };

    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);
    // let mut traveresed_arms = quote! {};
    // for arm in arms {
    //     let traversed_arm = quote_as_arm(arm, enclosing_item_attr_args);
    //     traveresed_arms = quote! { #traveresed_arms #traversed_arm }
    // }

    quote! { #(#new_attrs)* #match_token #expr { #arms } }
    // quote! { #(#attrs)* #match_token #expr { #traveresed_arms } }
}

/// Handles a method call expression: `x.foo::<T>(a, b)`.
fn quote_as_expr_method_call(
    expr_method_call: &syn::ExprMethodCall,
    enclosing_item_attr_args: &AttrArgs,
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

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_method_call }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_method_call };
    //     }
    // }

    let (receiver, args) = if loggable_found {
        (quote! { #receiver }, quote! { #args }) // TODO: Test.
    } else {
        let receiver = quote_as_expr(&**receiver, None, enclosing_item_attr_args);
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // let turbofish = match turbofish {
        //     Some(angle_bracketed_generic_arguments) =>
        //         Some(quote_as_angle_bracketed_generic_arguments(angle_bracketed_generic_arguments, enclosing_item_attr_args)),
        //     _ => turbofish
        // };
        let mut traversed_args = quote! {};
        for arg in args {
            let traversed_arg = quote_as_expr(arg, None, enclosing_item_attr_args);
            traversed_args = quote! { #traversed_args #traversed_arg, }
        }
        (receiver, traversed_args)
    }; // TODO: Test.

    // let receiver = quote_as_expr(&**receiver, None, enclosing_item_attr_args);
    // // // Likely not applicable for instrumenting the run time functions and
    // // // closures (as opposed to compile time const functions and closures).
    // // let turbofish = match turbofish {
    // //     Some(angle_bracketed_generic_arguments) =>
    // //         Some(quote_as_angle_bracketed_generic_arguments(angle_bracketed_generic_arguments, enclosing_item_attr_args)),
    // //     _ => turbofish
    // // };
    // let mut traversed_args = quote! {};
    // for arg in args {
    //     let traversed_arg = quote_as_expr(arg, None, enclosing_item_attr_args);
    //     traversed_args = quote! { #traversed_args #traversed_arg, }
    // }

    quote! { #(#new_attrs)* #receiver #dot_token #method #turbofish ( #args ) }
    // quote! { #(#attrs)* #receiver #dot_token #method #turbofish ( #traversed_args ) }
}

/// Handles a parenthesized expression: `(a + b)`.
fn quote_as_expr_paren(
    expr_paren: &syn::ExprParen,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprParen { // A parenthesized expression: `(a + b)`.
        attrs, //: Vec<Attribute>,
        // paren_token, //: Paren,
        expr, //: Box<Expr>,
        .. // paren_token
    } = expr_paren;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_paren }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_paren };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* ( #expr ) }
    // quote! { #(#attrs)* ( #expr ) }
}

/// Handles a path like `core::mem::replace` possibly containing generic parameters
/// and a qualified self-type (`<T as Display>::fmt`).
///
/// ### Parameters
/// * If the last path segment is `"_[e]print"` and the second parameter `is_print_func_name.is_some()`
///   then updates the `is_print_func_name` parameter with the value `Option<&true>`.  
///   TODO: .. -> 'If the path is `"[std::io::]_[e]print"` or `"[std::][e]print"`' (see `[e]println()` macro definition).
///   TODO: Consider checking `is_print_func_name.is_some()` first.
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
            // TODO: See also `std::[e]print()` in `[e]println()` macro definition.
            // TODO: Validate the path segments prior to the last one (those are the `std::io::` only).
            if let Some(is_print_func_name) = is_print_func_name {
                *is_print_func_name = true;
            }
        }
    }
    quote! { #expr_path }
}

/// Handles a range expression: `1..2`, `1..`, `..2`, `1..=2`, `..=2`.
fn quote_as_expr_range(
    expr_range: &syn::ExprRange,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprRange {
        attrs,  //: Vec<Attribute>,
        start,  //: Option<Box<Expr>>,
        limits, //: RangeLimits,
        end,    //: Option<Box<Expr>>,
    } = expr_range;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_range }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_range };
    //     }
    // }

    let (start, end) = if loggable_found {
        (Some(quote! { #start }), Some(quote! { #end })) // TODO: Test.
    } else {
        let start = start
            .as_ref()
            .map(|start| quote_as_expr(&**start, None, enclosing_item_attr_args));
        let end = end
            .as_ref()
            .map(|end| quote_as_expr(&**end, None, enclosing_item_attr_args));
        (start, end) // TODO: Test.
    };
    // let start = start
    //     .as_ref()
    //     .map(|start| quote_as_expr(&**start, None, enclosing_item_attr_args));
    // let end = end
    //     .as_ref()
    //     .map(|end| quote_as_expr(&**end, None, enclosing_item_attr_args));

    quote! { #(#new_attrs)* #start #limits #end }
    // quote! { #(#attrs)* #start #limits #end }
}

/// Handles an address-of operation: `&raw const place` or `&raw mut place`.
fn quote_as_expr_raw_addr(
    expr_raw_addr: &syn::ExprRawAddr,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprRawAddr {
        attrs,      //: Vec<Attribute>,
        and_token,  //: And,
        raw,        //: Raw,
        mutability, //: PointerMutability,
        expr,       //: Box<Expr>,
    } = expr_raw_addr;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_raw_addr }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_raw_addr };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #and_token #raw #mutability #expr }
    // quote! { #(#attrs)* #and_token #raw #mutability #expr }
}

/// Handles a referencing operation: `&a` or `&mut a`.
fn quote_as_expr_reference(
    expr_reference: &syn::ExprReference,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprReference {
        attrs,      //: Vec<Attribute>,
        and_token,  //: And,
        mutability, //: Option<Mut>,
        expr,       //: Box<Expr>,
    } = expr_reference;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_reference }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_reference };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #and_token #mutability #expr }
    // quote! { #(#attrs)* #and_token #mutability #expr }
}
fn quote_as_expr_repeat(
    expr_repeat: &syn::ExprRepeat,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprRepeat { // [0u8; N]
        attrs, //: Vec<Attribute>,
        // bracket_token, //: Bracket,
        expr, //: Box<Expr>,
        semi_token, //: Semi,
        len, //: Box<Expr>,
        .. // bracket_token
    } = expr_repeat;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_repeat }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_repeat };
    //     }
    // }

    let (expr, len) = if loggable_found {
        (quote! { #expr }, quote! { #len }) // TODO: Test.
    } else {
        let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args); // TODO: Test.
        let len = quote_as_expr(&**len, None, enclosing_item_attr_args); // TODO: Test.
        (expr, len)
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);
    // let len = quote_as_expr(&**len, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* [ #expr #semi_token #len ] }
    // quote! { #(#attrs)* [ #expr #semi_token #len ] }
}

/// Handles a `return`, with an optional value to be returned.
fn quote_as_expr_return(
    expr_return: &syn::ExprReturn,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprReturn {
        attrs,        //: Vec<Attribute>,
        return_token, //: Return,
        expr,         //: Option<Box<Expr>>,
    } = expr_return;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_return }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_return };
    //     }
    // }
    let expr = expr.as_ref().map(|expr| {
        if loggable_found {
            quote! { #expr } // TODO: Test.
        } else {
            quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
        }
        // quote_as_expr(&**expr, None, enclosing_item_attr_args)
    });

    quote! { #(#new_attrs)* #return_token #expr }
    // quote! { #(#attrs)* #return_token #expr }
}

fn quote_as_field_value(field: &syn::FieldValue, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let syn::FieldValue {
        attrs,       //: Vec<Attribute>,
        member,      //: Member,
        colon_token, //: Option<Colon>,
        expr,        //: Expr,
    } = field;

    // NOTE: Field values in the struct literals cannot be `#[[non_]loggable]`:
    // ```
    // Compiler Error:
    //     expected non-macro attribute, found attribute macro `loggable`
    //     not a non-macro attribute
    // ```
    // Thus the attr_args combining is not applicable.
    //
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #field };
    //     }
    // }
    let expr = quote_as_expr(expr, None, attr_args);
    quote! { #(#attrs)* #member #colon_token #expr }
}

/// Handles a struct literal expression: `Point { x: 1, y: 1 }`.
/// The `rest` provides the value of the remaining fields as in `S { a: 1, b: 1, ..rest }`.
fn quote_as_expr_struct(
    expr_struct: &syn::ExprStruct,
    enclosing_item_attr_args: &AttrArgs,
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

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_struct }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_struct };
    //     }
    // }

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

    let (fields, rest) = if loggable_found {
        (
            quote! { #fields },                         // TODO: Test.
            rest.as_ref().map(|expr| quote! { #expr }), // TODO: Test.
        )
    } else {
        let fields = {
            let mut traversed_fileds = quote! {};
            for field in fields {
                let traversed_field = quote_as_field_value(field, enclosing_item_attr_args);
                traversed_fileds = quote! { #traversed_fileds #traversed_field, };
            }
            traversed_fileds
        };
        let rest = rest
            .as_ref()
            .map(|expr| quote_as_expr(&**expr, None, enclosing_item_attr_args));
        (fields, rest) // TODO: Test.
    };
    // let fields = {
    //     let mut traversed_fileds = quote! {};
    //     for field in fields {
    //         let traversed_field = quote_as_field_value(field, enclosing_item_attr_args);
    //         traversed_fileds = quote! { #traversed_fileds #traversed_field, };
    //     }
    //     traversed_fileds
    // };
    // let rest = rest
    //     .as_ref()
    //     .map(|expr| quote_as_expr(&**expr, None, enclosing_item_attr_args));

    quote! { #(#new_attrs)* #qself_and_apth { #fields #dot2_token #rest } }
    // quote! { #(#attrs)* #qself_and_apth { #fields #dot2_token #rest } }
}

/// Handles a try-expression: `expr?`.
fn quote_as_expr_try(
    expr_try: &syn::ExprTry,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprTry {
        attrs,          //: Vec<Attribute>,
        expr,           //: Box<Expr>,
        question_token, //: Question,
    } = expr_try;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_try }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_try };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #expr #question_token }
    // quote! { #(#attrs)* #expr #question_token }
}

/// Handles a `try` block: `try { ... }`.
///
/// NOTE: The `try` blocks are
/// [unstable](https://doc.rust-lang.org/unstable-book/language-features/try-blocks.html)
/// (see [also](https://doc.rust-lang.org/std/ops/trait.Try.html)).
fn quote_as_expr_try_block(
    expr_try_block: &syn::ExprTryBlock,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprTryBlock {
        attrs,     //: Vec<Attribute>,
        try_token, //: Try,
        block,     //: Block,
    } = expr_try_block;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_try_block }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_try_block };
    //     }
    // }

    let block = if loggable_found {
        quote! { #block } // TODO: Test.
    } else {
        quote_as_block(block, enclosing_item_attr_args) // TODO: Test.
    };
    // let block = quote_as_block(block, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #try_token #block }
    // quote! { #(#attrs)* #try_token #block }
}

/// Handles a tuple expression: `(a, b, c, d)`.
fn quote_as_expr_tuple(
    expr_tuple: &syn::ExprTuple,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprTuple {
        attrs, //: Vec<Attribute>,
        // paren_token, //: Paren,
        elems, //: Punctuated<Expr, Comma>,
        .. // paren_token
    } = expr_tuple;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_tuple }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_tuple };
    //     }
    // }

    let elems = if loggable_found {
        quote! { #elems } // TODO: Test.
    } else {
        let mut traversed_elems = quote! {};
        for elem in elems {
            let traversed_elem = quote_as_expr(elem, None, enclosing_item_attr_args);
            traversed_elems = quote! { #traversed_elems #traversed_elem, }
        }
        traversed_elems // TODO: Test.
    };
    // let elems = {
    //     let mut traversed_elems = quote! {};
    //     for elem in elems {
    //         let traversed_elem = quote_as_expr(elem, None, enclosing_item_attr_args);
    //         traversed_elems = quote! { #traversed_elems #traversed_elem, }
    //     }
    //     traversed_elems
    // };

    quote! { #(#new_attrs)*( #elems ) }
    // quote! { #(#attrs)*( #elems ) }
}

/// Handles a unary operation: `!x`, `*x`.
fn quote_as_expr_unary(
    expr_unary: &syn::ExprUnary,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprUnary {
        // `!x`, `*x`
        attrs, //: Vec<Attribute>,
        op,    //: UnOp,
        expr,  //: Box<Expr>,
    } = expr_unary;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_unary }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_unary };
    //     }
    // }

    let expr = if loggable_found {
        quote! { #expr } // TODO: Test.
    } else {
        quote_as_expr(&**expr, None, enclosing_item_attr_args) // TODO: Test.
    };
    // let expr = quote_as_expr(&**expr, None, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #op #expr }
    // quote! { #(#attrs)* #op #expr }
}

/// Handles an unsafe block: `unsafe { ... }`.
fn quote_as_expr_unsafe(
    expr_unsafe: &syn::ExprUnsafe,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprUnsafe {
        // unsafe { ... }
        attrs,        //: Vec<Attribute>,
        unsafe_token, //: Unsafe,
        block,        //: Block,
    } = expr_unsafe;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_unsafe }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_unsafe };
    //     }
    // }

    let block = if loggable_found {
        quote! { #block } // TODO: Test.
    } else {
        quote_as_block(block, enclosing_item_attr_args) // TODO: Test.
    };
    // let block = quote_as_block(block, enclosing_item_attr_args);

    quote! { #(#new_attrs)* #unsafe_token #block }
    // quote! { #(#attrs)* #unsafe_token #block }
}

/// Handles a `while` loop: `while expr { ... }`.
fn quote_as_expr_while(
    expr_while: &syn::ExprWhile,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprWhile {
        attrs,       //: Vec<Attribute>,
        label,       //: Option<Label>,
        while_token, //: While,
        cond,        //: Box<Expr>,
        body,        //: Block,
    } = expr_while;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_while }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_while };
    //     }
    // }

    let (cond, body) = if loggable_found {
        (quote! { #cond }, quote! { #body }) // TODO: Test.
    } else {
        (
            quote_as_expr(&**cond, None, enclosing_item_attr_args), // TODO: Test.
            quote_as_loop_block(body, enclosing_item_attr_args),    // TODO: Test.
        )
    };
    // let cond = quote_as_expr(&**cond, None, enclosing_item_attr_args);
    // let body = quote_as_loop_block(body, enclosing_item_attr_args);

    let extra_borrow = if cfg!(feature = "single_threaded") {
        quote! { .borrow_mut() } // TODO: Test with `#[cfg(feature = "single_threaded")]`, either update or document.
    } else {
        quote! {}
    };
    quote! {
        {
            // At the moment of writing the unit value `()`
            // is the only known possible value returnable by `while` loop.
            let ret_val = #(#new_attrs)* #label #while_token #cond #body ;

            fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger|
                logger.borrow_mut() #extra_borrow .log_loop_end());

            ret_val
        }
    }
}

/// Handles a `yield` expression: `yield expr`.
fn quote_as_expr_yield(
    expr_yield: &syn::ExprYield,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ExprYield {
        attrs,       //: Vec<Attribute>,
        yield_token, //: Yield,
        expr,        //: Option<Box<Expr>>,
    } = expr_yield;

    let (new_attrs, non_loggable_found, loggable_found) =
        updated_loggable_attr_args(attrs, enclosing_item_attr_args);
    if non_loggable_found {
        return quote! { #expr_yield }; // TODO: Test.
    }
    // for attr in attrs {
    //     if attr.is_traverse_stopper() {
    //         return quote! { #expr_yield };
    //     }
    // }

    let expr = expr.as_ref().map(|ref_boxed_expr| {
        if loggable_found {
            quote! { #expr } // TODO: Test.
        } else {
            quote_as_expr(&**ref_boxed_expr, None, enclosing_item_attr_args) // TODO: Test.
        }
        // quote_as_expr(&**ref_boxed_expr, None, enclosing_item_attr_args)
    });

    quote! { #(#new_attrs)* #yield_token #expr }
    // quote! { #(#attrs)* #yield_token #expr }
}

// NOTE:
// ```txt
// Compiler Error:
//   attributes on expressions are experimental
//   see issue #15701 <https://github.com/rust-lang/rust/issues/15701> for more information
//   add `#![feature(stmt_expr_attributes)]` to the crate attributes to enable
// ```
// In other words,
// * by default the expressions cannot be `#[loggable]`,
//   and any code
//      combining the attr_args of the expression's `#[loggable]`
//      with those of the enclosing entity's `#[loggable]`
//   is a dead code;
// * but if the user has `#![feature(stmt_expr_attributes)]` and tries to make the expressions `#[loggable]`,
//   or the attributes on expressions stop being experimental,
//   then the dead code is expected { to come into play and do the expected thing }.
// That is why the combining is implemented, even though it is dead by default.
// TODO: Test in a separate file/crate with the `#![feature(stmt_expr_attributes)]`.
//
/// Handles the expressions.
/// 
/// ### Parameters
/// * If the expression is `syn::ExprPath` and
///   the last path segment is `"_[e]print"` and the second parameter `is_print_func_name.is_some()` 
///   then updates the `is_print_func_name` parameter with the value `Option<&true>`.
///   See more details on `quote_as_expr_path()`.

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
