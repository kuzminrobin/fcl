use quote::quote;

mod exprs;
mod items;

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
///
/// <br>
///
/// ## Parameters
///
/// ### No attribute macro parameters
/// The `#[loggable]` attribute with no parameters has the same effect as `#[loggable(log_params)]`,
/// i.e., the function and closure parameters are logged by default.
///
/// ### `log_params` (default, optional)
/// Log the parameters in the annotated entity and its internal entities recursively.
///
/// ### `skip_params` (optional)
/// Skip the parameters logging in the annotated entity and its internal entities recursively.  
///
/// If the (directly or recursively) annotated function or closure has no parameters
/// then its parameter block will be logged as `()`, otherwise `(..)`.
///
/// ### Examples
/// ```compile_fail, E0432, E0433, E0599
/// use fcl_proc_macros::loggable;
///
/// #[loggable] // Log by defualt the function and closure parameters inside of module `m` recursively
/// mod m {     // (and add prefix "m::" to the function and closure names).
///     use fcl_proc_macros::loggable;
///
///     pub fn f(b: bool) {        // Log example: `m::f(b: true) {`.
///                                // The parameter `b: true` is logged by default.
///         Some(5).map(|x| x + 1);// Log example: `  m::f()::closure{168,29:168,33}(x: 5) {} -> 6`.
///                                // The closure parameter `x: 5` is logged by default.
///     }
///     #[loggable(skip_params)]   // Skip the parameters logging for `g()` and its internals
///                                // (and clear the prefix "m::" (TODO: Prevent clearing)).
///     pub fn g(p: u8) {          // Logs: `g(..) {`. The parameter `p` is not logged,
///                                // the `..` instead tells that `g()` has parameter(s).
///         Some(p).map(|x| x + 2);// Log example: `g()::closure{176,29:176,37}(..) {} -> 3`.
///                                // The closure parameter `x` is not logged (the `..` instead).
///         #[loggable(log_params)]// Log the parameters for `h()` and its internals
///                                // (and clear the prefix "g()::" (TODO: Prevent clearing)).
///         fn h(ph: u8) {         // Log example: `h(ph: 1) {`. The parameter `ph: 1` is logged.
///             Some(ph).map(|y| y + 3); // `h()::closure{180,34:180,42}(y: 1) {} -> 4`.
///                                // The parameter `y: 1` is logged.
///         }
///
///         h(p);                  // Call `h()` from `g()`.
///     }
/// }
/// // Call the instrumented functions to generate the FCL log:
/// m::f(true);
/// m::g(1)
/// ```
/// <br>
///
/// ### `prefix` (optional)
/// Is unlikely to be used by the user.
///
/// Sets the name prefix for the annotated entity and its internals recursively.
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
    let attr_args = syn::parse_macro_input!(attr_args_ts as AttrArgs); // Handles the compilation errors appropriately (checked).
    let output = {
        if let Ok(item) = syn::parse::<syn::Item>(attributed_item.clone()) {
            items::quote_as_item(&item, &attr_args, true)
        } else if let Ok(expr) = syn::parse::<syn::Expr>(attributed_item.clone()) {
            exprs::quote_as_expr(&expr, None, &attr_args)
        } else {
            // TODO: Is it still applicable? Explain.
            let closure_w_opt_comma =
                syn::parse_macro_input!(attributed_item as ExprClosureWOptComma); // Handles the compilation errors appropriately.
            exprs::quote_as_expr_closure(&closure_w_opt_comma.closure, &attr_args)
        }
    };
    output.into()
}

#[proc_macro_attribute]
pub fn loggable_block_contents(
    attr_args_ts: proc_macro::TokenStream,
    attributed_item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr_args = syn::parse_macro_input!(attr_args_ts as AttrArgs); // NOTE: Handles the compilation errors appropriately (checked).
    let item_fn = syn::parse_macro_input!(attributed_item as syn::ItemFn);
    // let item_mod = syn::parse_macro_input!(attributed_item as syn::ItemMod);
    items::quote_as_item_fn_loggable_block_contents(&item_fn, &attr_args /*, true */).into()
    // items::quote_as_item_mod_loggable_block_contents(&item_fn, &attr_args/*, true */).into()

    // let syn::ItemMod {
    //     attrs,     // : Vec<Attribute>,
    //     vis,       // : Visibility,
    //     unsafety,  // : Option<Unsafe>,
    //     mod_token, // : Mod,
    //     ident,     // : Ident,
    //     content,   // : Option<(Brace, Vec<Item>)>,
    //     semi,      // : Option<Semi>,
    // } = item_mod;

    // if ident.to_string() != "loggable_block_contents" {
    //     return syn::Error::new(
    //         ident.span(),
    //         format!("expected `{}`", "loggable_block_contents"), // TODO: loggable_block_contents to consts file.
    //     )
    //     .into_compile_error()
    //     .into();
    //     // syn::Error::into_compile_error()
    // }
    // let mut new_attrs = vec![];
    // let mut has_loggable = false;

    // for attr in attrs {
    //     if attr.is_non_loggable() { // TODO: Consider the arbitrtary combination of [multiple] `#[loggable]` and `#[non_loggable]` for general case and this case. What's reasonable in general, what's the difference here and why.
    //         return quote! { #item_mod }.into();
    //     }
    //     new_attrs.push(items::if_loggable_then_combine_attr_args(
    //         attr,
    //         &mut has_loggable,
    //         enclosing_item_attr_args,
    //     ));
    // }

    // /*
    //     let content = if has_loggable {
    //         // // NOTE: The impl below deosn't compile since either a trait or a type have to be locally defined.
    //         // impl quote::ToTokens for (syn::token::Brace, Vec<syn::Item>) {
    //         //     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    //         //         let (.., items) = self;
    //         //         let mut tokenized_items = quote! {};
    //         //         for item in items {
    //         //             tokenized_items = quote! { #tokenized_items #item };
    //         //         }
    //         //         *tokens = quote! { { #tokenized_items } }
    //         //     }
    //         // }
    //         //
    //         // quote! { #content }

    //         // No item traversing. The items are passed as they are.
    //         let content = content.as_ref().map(|(_brace, items)| {
    //             quote! { { #(#items)* } }
    //             // let mut copied_items = quote! {};
    //             // for item in items {
    //             //     copied_items = quote! { #copied_items #item };
    //             // }
    //             // quote! { { #copied_items } }
    //         });
    //         content
    //     } else {
    //         let attr_args = AttrArgs {
    //             prefix: if enclosing_item_attr_args.prefix.is_empty() {
    //                 quote! { #ident }
    //             } else {
    //                 let prefix = &enclosing_item_attr_args.prefix;
    //                 quote! { #prefix::#ident }
    //             },
    //             ..*enclosing_item_attr_args
    //         };

    //         // Traverse the items:
    //         let content = content.as_ref().map(|(_brace, items)| {
    //             let mut traversed_items = quote! {};
    //             for item in items {
    //                 let item = quote_as_item(item, &attr_args, false);
    //                 traversed_items = quote! { #traversed_items #item };
    //             }
    //             quote! { { #traversed_items } }
    //         });
    //         content
    //     };
    //     quote! { #(#new_attrs)* #vis #unsafety #mod_token #ident #content #semi }
    // */
    // /*
    // fn quote_as_item_mod(
    //     item_mod: &syn::ItemMod,
    //     enclosing_item_attr_args: &AttrArgs,
    // ) -> proc_macro2::TokenStream {
    //     let syn::ItemMod {
    //         attrs,     //: Vec<Attribute>,
    //         vis,       //: Visibility,
    //         unsafety,  //: Option<Unsafe>,
    //         mod_token, //: Mod,
    //         ident,     //: Ident,
    //         content,   //: Option<(Brace, Vec<Item>)>,
    //         semi,      //: Option<Semi>,
    //     } = item_mod;

    // }

    // */
    // quote! {}.into()

    // match syn::parse::<syn::Item/*::Mod*/>(attributed_item) {
    //     Ok(item) => {
    //         match item {
    //             syn::Item::Mod(item_mod) =>
    //                 items::quote_as_item_mod_loggable_block_contents(&item, &attr_args/*, true */).into()
    //                 // quote_as_item_mod(item_mod, enclosing_item_attr_args)
    //                 ,
    //             _ => proc_macro::TokenStream::from(err.to_compile_error())
    //         }

    //     },
    //     Err(err) => return proc_macro::TokenStream::from(err.to_compile_error()),
    // }

    // let output = {
    //     if let Ok(item) = syn::parse::<syn::Item>(attributed_item.clone()) {
    //         items::quote_as_item(&item, &attr_args, true)
    //     } else if let Ok(expr) = syn::parse::<syn::Expr>(attributed_item.clone()) {
    //         exprs::quote_as_expr(&expr, None, &attr_args)
    //     } else {
    //         let closure_w_opt_comma =
    //             syn::parse_macro_input!(attributed_item as ExprClosureWOptComma); // NOTE: Handles the compilation errors appropriately.
    //         exprs::quote_as_expr_closure(&closure_w_opt_comma.closure, &attr_args)
    //     }
    // };
    // output.into()
}

/// Runs through the collection of `current_attrs` parameter, 
/// for each `#[loggable]` attribute combines its arguments with `enclosing_item_attr_args` in a new attribute, 
/// clones all other attributes,
/// collects all the attributes in the output collection.
/// 
/// Returns a tuple of
/// * the vector of attributes where for each `#[loggable]` attribute the arguments are combined;
/// * `true`, if the current entity has a `#[non_loggable]` attribute,
///   `false` otherwise;
/// * `true`, if the current entity has `#[loggable]` (with or without arguments) among its attributes,
///   `false` otherwise.
fn updated_loggable_attr_args(
    current_attrs: &Vec<syn::Attribute>,
    enclosing_item_attr_args: &AttrArgs,
) -> (
    Vec<syn::Attribute>, // `updated_attrs`
    bool, // `non_loggable_found`
    bool, // `loggable_found`
) {
    let mut new_attrs = vec![];
    let mut has_loggable = false;
    let mut non_loggable_found = false;

    for attr in current_attrs {
        if attr.is_non_loggable() {
            non_loggable_found = true;
        }
        // if attr.is_non_loggable() {
        //     new_attrs.push(attr.clone());
        //     // return (false, current_attrs.clone(), has_loggable);
        //     // return (false, new_attrs, has_loggable);
        // } else {
            new_attrs.push(if_loggable_then_combine_attr_args(
                attr,
                enclosing_item_attr_args,
                &mut has_loggable,
            ));
        // }
    }
    (new_attrs, non_loggable_found, has_loggable)
    // (true, new_attrs, has_loggable)
}

/// Combines the parameters 
/// and returns a token stream of `prefix = <prefix>, (log|skip)_params, (log|skip)_closure_coords,`.
fn combine_loggable_attr_params_as_meta_tokens(
    user_provided_attr_info: &LoggableAttrInfo,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let LoggableAttrInfo {
        prefix: user_provided_prefix,
        params_logging: user_provided_params_logging,
        log_closure_coords: user_provided_log_closure_coords,
    } = user_provided_attr_info;

    let new_prefix = user_provided_prefix
        .as_ref()
        .unwrap_or(&enclosing_item_attr_args.prefix);
    // println!("prefix: \"{:?}\"", new_prefix);
    let mut updated_tokens = quote! { prefix = #new_prefix, };
    // println!("updated_tokens: {:?}", updated_tokens);
    // if let Some(specified_prefix) = prefix {
    //     updated_tokens = quote!{ prefix = #specified_prefix, };
    // } else {
    //     let inherited_prefix = &attr_args.prefix;
    //     updated_tokens = quote!{ prefix = #inherited_prefix, };
    // }

    let new_params_logging =
        user_provided_params_logging.unwrap_or(enclosing_item_attr_args.params_logging);
    match new_params_logging {
        ParamsLogging::Log => updated_tokens = quote! { #updated_tokens log_params, },
        ParamsLogging::Skip => updated_tokens = quote! { #updated_tokens skip_params, },
        // Any new ones require attention.
    }
    // println!("log_closure_coords: {:?}", log_closure_coords);
    let new_log_closure_coords =
        user_provided_log_closure_coords.unwrap_or(enclosing_item_attr_args.log_closure_coords);
    if new_log_closure_coords {
        updated_tokens = quote! { #updated_tokens log_closure_coords, }
    } else {
        updated_tokens = quote! { #updated_tokens skip_closure_coords, }
    }
    // println!("updated_tokens: {:?}", updated_tokens);
    updated_tokens
}

// TODO: Review/refactor and doc-comment.
//
/// If the `attr` parameter is not `[fcl_proc_macros::]loggable` then returns a clone of it.
/// 
/// Otherwise
/// * writes `true` to the value referred to by the `is_loggable` parameter;
/// * returns a new `syn::Attribute` instance containing `#[loggable(<args>)]` with arguments that are a combination of 
///   * `attr` parameter's arguments
///   * `enclosing_item_attr_args`.
fn if_loggable_then_combine_attr_args(
    attr: &syn::Attribute,
    enclosing_item_attr_args: &AttrArgs,
    is_loggable: &mut bool,
) -> syn::Attribute {

    // If `attr` is `#[loggable]` (with or without args):
    if let Some(user_provided_attr_info) = attr.get_loggable_attr_info() {
        // TODO: The comment below to mdBook.
        // The `attr` is the `#[loggable..]` attribute. In other words
        // the fact that the `#[loggable..]` attribute is among the attributes of (is found by) the caller of this function
        // means that the caller of this function has been called as part of the recursive traverse
        // (as opposed to a separate macro-expansion of its own `#[loggable..]` attribute).
        // That is, the caller of this function must not recursively traverese one's own internals,
        // it must just retain its `#[loggable..]` attribute (and leave the internals as they are).
        // That `#[loggable..]` attribute will be macro-expanded as a separate invocation of fn `fcl_proc_macros::loggable()`;
        // that's when that `#[loggable..]` attribute will not be among the attributes of (will not be found by) the caller of this function,
        // and that's when the internals (of the caller of this function) will be traversed and instrumented recursively.
        *is_loggable = true;

        // Combine the attr args in a new attr (`#[loggable(<combined_attr_args>)]`):
        let new_attr = syn::Attribute {
            pound_token: attr.pound_token,
            style: attr.style,
            bracket_token: attr.bracket_token,
            meta: match &attr.meta {
                // The `attr` parameter is `#[loggable(<args>)]`.
                // Generate the `loggable(<combined_attr_args>)` meta:
                syn::Meta::List(metalist) => syn::Meta::List(syn::MetaList {
                    path: metalist.path.clone(),
                    delimiter: metalist.delimiter.clone(),
                    tokens: {
                        combine_loggable_attr_params_as_meta_tokens(
                            &user_provided_attr_info,
                            enclosing_item_attr_args,
                        )
                    },
                }),
                // The `attr` parameter is `#[loggable]` with no args.
                // Generate the `loggable(<enclosing_item_attr_args>)` meta:
                syn::Meta::Path(path) => syn::Meta::List(syn::MetaList {
                    path: path.clone(),
                    delimiter: syn::MacroDelimiter::Paren(Default::default()),
                    tokens: {
                        combine_loggable_attr_params_as_meta_tokens(
                            &user_provided_attr_info,
                            enclosing_item_attr_args,
                        )
                    },
                }),
                _ => attr.meta.clone(),
            },
        };
        // println!("new_attr: {:?}", new_attr.meta.);
        new_attr
    } else {
        attr.clone()
    }
}

// TODO: Consider deduping the `LoggableAttrInfo` and `LoggableAttrArgsOpt` or explaining why not.
struct LoggableAttrInfo {
    prefix: Option<proc_macro2::TokenStream>, //Option<String>,
    params_logging: Option<ParamsLogging>,
    log_closure_coords: Option<bool>,
}

struct LoggableAttrArgsOpt {
    prefix: Option<proc_macro2::TokenStream>, // Option<String>,
    params_logging: Option<ParamsLogging>,
    log_closure_coords: Option<bool>,
}
impl syn::parse::Parse for LoggableAttrArgsOpt {
    /// Upon successful parsing returns `syn::Result::ok(Self)`, where in `Self`
    /// * `prefix` is `Some(<Path>)` if the `input` contains `prefix=Path`, otherwise `None`;
    /// * `params_logging`  
    ///   * is `Some(ParamsLogging::Skip)` if the `input` contains `skip_params`,
    ///   * is `Some(ParamsLogging::Log)` if the `input` contains `log_params`,
    ///   * is `None` otherwise;
    /// * `log_closure_coords`
    ///   * is `Some(false)` if the `input` contains `skip_closure_coords`,
    ///   * is `Some(true)` if the `input` contains `log_closure_coords`,
    ///   * is `None` otherwise.
    /// 
    /// Otherwise returns `syn::Result::err(e)`.
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = LoggableAttrArgsOpt {
            prefix: None,
            params_logging: None,
            log_closure_coords: None,
        };

        //println!("input: {}", input);

        // println!("input2: {}", input);
        loop {
            // if content.is_empty() {
            if input.is_empty() {
                break;
            }
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
                continue;
            } else if lookahead.peek(kw::prefix) {
                input.parse::<kw::prefix>()?;
                input.parse::<syn::Token![=]>()?;
                let optional_prefix = input.parse::<QSelfOrPath>()?; // TODO: Consider the case `#[loggable(prefix = )]` (clearing the prefix).
                if let QSelfOrPath(Some(q_self_or_path)) = optional_prefix {
                    let prefix_ts = match q_self_or_path {
                        LogPrefix::QSelf(qself) => quote! { #qself },
                        LogPrefix::Path(path) => quote! { #path },
                        // LogPrefix::Empty => quote! {},   // (cledaring the prefix)
                    };
                    args.prefix = Some(prefix_ts); //Some(remove_spaces(&prefix_ts.to_string()));
                }
            } else if lookahead.peek(kw::skip_params) {
                input.parse::<kw::skip_params>()?;
                args.params_logging = Some(ParamsLogging::Skip);
            } else if lookahead.peek(kw::log_params) {
                input.parse::<kw::log_params>()?;
                args.params_logging = Some(ParamsLogging::Log);
            } else if lookahead.peek(kw::skip_closure_coords) {
                input.parse::<kw::skip_closure_coords>()?;
                args.log_closure_coords = Some(false);
                // println!("args.log_closure_coords: {:?}", args.log_closure_coords);
            } else if lookahead.peek(kw::log_closure_coords) {
                input.parse::<kw::log_closure_coords>()?;
                args.log_closure_coords = Some(true);
            } else {
                return Err(lookahead.error());
            }
        }
        Ok(args)
    }
}

// fn is_fcl_attribute(attr: &syn::Attribute, attr_name: &str) -> bool {
//     let path = match &attr.meta {
//         syn::Meta::Path(path) => path,
//         syn::Meta::List(syn::MetaList { path, .. }) => path,
//         _ => return false,
//     };
//     // If the last path segment equals `attr_name` // e.g. "non_loggable"
//     //      && preceeding path segment is None or is "fcl_proc_macros"
//     // then
//     //      return true // is `non_loggable`
//     // return false // is not `non_loggable` or is user's own `non_loggable` (`<user's_path>::non_loggable`).
//     if let Some(last_path_segment) = path.segments.last()
//         && last_path_segment.ident.to_string() == attr_name
//         && (path.segments.len() < 2 || {
//             let prev_segment_idx = path.segments.len() - 2;
//             path.segments[prev_segment_idx].ident.to_string() == "fcl_proc_macros"  // TODO: "fcl_proc_macros" to a file of consts.
//         })
//     {
//         return true;
//     }
//     return false;
// }

/* TODO: -> ExtendedAttribute? */
trait FclAttribute {
    fn get_loggable_attr_info(&self) -> Option<LoggableAttrInfo>;
    fn is_fcl_attribute(&self, attr_name: &str) -> bool;
    fn is_non_loggable(&self) -> bool;
}
impl FclAttribute for syn::Attribute {
    /// Returns
    /// * `true` if `self` { equals `attr_name` parameter or ends with `fcl_proc_macros::<attr_name>` },
    /// * `false` otherwise.
    // NOTE: Contains/assumes "fcl_proc_macros". Not to be renamed to `is()`.
    fn is_fcl_attribute(&self, attr_name: &str) -> bool {
        // fn is_fcl_attribute(attr: &syn::Attribute, attr_name: &str) -> bool {
        let path = match &self.meta {
            // let path = match &attr.meta {
            syn::Meta::Path(path) => path,
            syn::Meta::List(syn::MetaList { path, .. }) => path,
            _ => return false,
        };

        if let Some(last_path_segment) = path.segments.last()
            && last_path_segment.ident.to_string() == attr_name
            && (path.segments.len() < 2 || {
                let prev_segment_idx = path.segments.len() - 2;
                path.segments[prev_segment_idx].ident.to_string() == "fcl_proc_macros" // TODO: "fcl_proc_macros" to a file of consts.
            })
        {
            return true;
        }
        return false;
    }

    /// Returns `None` if `self` is not `[fcl_proc_macros::]loggable`.
    /// Otherwise returns `Some(LoggableAttrInfo)` where the fields are
    /// * the same as in `LoggableAttrArgsOpt` upon successful parsing, including all `None` if `loggable` args are empty,
    ///   (see comments for `fcl_proc_macros::LoggableAttrArgsOpt::parse()`),
    /// * all `None` upon parsing gailure (at the moment of writing; TODO: Review in details
    ///   and resolve or doc the parsing failure case. See a NOTE in the code).
    fn get_loggable_attr_info(&self) -> Option<LoggableAttrInfo> {
        let (path, optional_tokens) = match &self.meta {
            syn::Meta::Path(path) => (path, None),
            syn::Meta::List(syn::MetaList { path, tokens, .. }) => (path, Some(tokens)),
            _ => return None,
        };

        let mut ret_val = None;

        // If the last path segment equals "loggable"
        //      && preceeding path segment is (None || ("fcl_proc_macros" (TODO: `&& is or ends up in the only one`)) )
        // then
        //      Get and return LoggableAttrInfo
        // return None
        if let Some(last_path_segment) = path.segments.last()
            && last_path_segment.ident.to_string() == "loggable"    // TODO: "loggable" to file of consts.
            && (path.segments.len() < 2 || {
                let prev_segment_idx = path.segments.len() - 2;
                path.segments[prev_segment_idx].ident.to_string() == "fcl_proc_macros" // TODO: "fcl_proc_macros" to file of consts.
            })
        {
            ret_val = Some(LoggableAttrInfo {
                prefix: None,             // Option<String>,
                params_logging: None,     // Option<ParamsLogging>,
                log_closure_coords: None, // Option<bool>,
            });
            if let Some(tokens) = optional_tokens {
                // println!("optional_tokens: {:?}", optional_tokens);
                if let Ok(parsed) = syn::parse2::<LoggableAttrArgsOpt>(tokens.clone()) {
                    ret_val = Some(LoggableAttrInfo {
                        prefix: parsed.prefix,
                        params_logging: parsed.params_logging,
                        log_closure_coords: parsed.log_closure_coords,
                    });
                }
                // TODO: `else`? Silently ignore the parsing error? NOTE: The parsing error 
                // could have been detected in the very beginning of `fn loggable()`; make sure[, test,] and doc it.
            }
        }
        return ret_val;
    }

    fn is_non_loggable(&self) -> bool {
        self.is_fcl_attribute("non_loggable")
        // <syn::Attribute as IsTraverseStopper>::is_fcl_attribute(self, "non_loggable")
    }
}

/// Removes spaces from a string, except around 'as' (in framgents like "\<MyType as MyTrait>").
///
/// Returns a copy of an argument with spaces removed, except around 'as'.
///
/// NOTE: If the argument contains sequences of '$as$', those will be replaced with ' as '.
///
/// ### Examples
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

fn update_param_data_from_pat(
    input_pat: &syn::Pat,
    param_format_str: &mut String,
    param_list: &mut proc_macro2::TokenStream,
) {
    match input_pat {
        // The Rust Reference. ClosureParam.
        // https://doc.rust-lang.org/reference/expressions/closure-expr.html#grammar-ClosureParam
        // https://doc.rust-lang.org/reference/patterns.html#grammar-PatternNoTopAlt
        // https://doc.rust-lang.org/reference/patterns.html#grammar-RangePattern

        // syn::Pat::Const(pat_const) => ?,
        // NOTE: Not found in The Rust Reference (links above) for PatternNoTopAlt.
        // NOTE: Example from ChatGPT looks too rare to fully parse the nested `block`:
        // |const [a, b, c]: [u8; 3]| { println!("{a} {b} {c}"); }
        syn::Pat::Ident(pat_ident) => {
            // x: f32
            let ident = &pat_ident.ident;
            param_format_str.push_str(&format!("{}: {{}}", ident)); // + "x: {}"
            *param_list = quote! { #param_list #ident.maybe_print(), } // + `x.maybe_print(), `
        }
        // syn::Pat::Lit(pat_lit) => ?,  // NOTE: Still questionable: Are literals applicable to params pattern?
        // The Rust Reference mentions/lists it but does not add clarity.
        // ChatGPT states "Not Applicable for params".

        // syn::Pat::Macro(pat_macro) => ?, // NOTE: Out of scope.
        // syn::Pat::Or(pat_or) => ?, // NOTE: Not found in The Rust Reference (for PatternNoTopAlt).
        syn::Pat::Paren(pat_paren) => {
            let syn::PatParen {
                // attrs, //: Vec<Attribute>,
                // paren_token, //: Paren,
                pat, //: Box<Pat>,
                ..
            } = pat_paren;
            param_format_str.push_str(&"(");
            update_param_data_from_pat(pat.as_ref(), param_format_str, param_list);
            param_format_str.push_str(&")");
        }
        // syn::Pat::Path(pat_path) => ?, // NOTE: Example is needed as a param (`path` without `: Type`).
        // syn::Pat::Range(pat_range) => ?, // NOTE: N/A as a param.
        syn::Pat::Reference(pat_reference) => {
            let syn::PatReference {
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
        // syn::Pat::Rest(pat_rest) => ?, // NOTE: N/A as a param.
        syn::Pat::Slice(pat_slice) => {
            let syn::PatSlice {
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
        syn::Pat::Struct(pat_struct) => {
            // struct MyPoint{ x: i32, y: i32}
            // fn f(MyPoint{x, y: _y}: MyPoint) {}
            // f(MyPoint{ x: 2, y: -4});  // Log: f(MyPoint { x: 2, y: _y: -4 }) {}
            let syn::PatStruct {
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
                let syn::FieldPat {
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
        syn::Pat::Tuple(pat_tuple) => {
            let syn::PatTuple {
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
        syn::Pat::TupleStruct(pat_tuple_struct) => {
            let syn::PatTupleStruct {
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
        syn::Pat::Type(pat_type) => {
            let syn::PatType {
                // attrs, //: Vec<Attribute>,
                pat, //: Box<Pat>,
                     // colon_token, //: Colon,
                     // ty, //: Box<Type>,
                ..
            } = pat_type;
            update_param_data_from_pat(pat.as_ref(), param_format_str, param_list);
        }
        // syn::Pat::Verbatim(token_stream) // Ignore unclear sequence of tokens among params.
        // syn::Pat::Wild(pat_wild) // Ignore `_` in the pattern.
        _ => {} // Do not print the param values.
    }
}

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

/// Closure with optional trailing comma
/// (when closure is the last argument of a function):
struct ExprClosureWOptComma {
    closure: syn::ExprClosure,
    _optional_comma: Option<syn::Token![,]>,
}
impl syn::parse::Parse for ExprClosureWOptComma {
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

    // ### Examples
    // `#[loggable(prefix = My::Path)]` // <MyStruct as MyPureTrait>
    syn::custom_keyword!(prefix);

    // Log the parameters in the annotated entity and its local entities recursively.
    // ### Examples
    // `#[loggable(log_params)]`
    syn::custom_keyword!(log_params);

    // Skip the parameters logging in the annotated entity and its local entities recursively.
    //
    // If the function has no prameters then its parameters block is logged as `()`, otherwise `(..)`.
    // ### Examples
    // `#[loggable(skip_params)]`
    syn::custom_keyword!(skip_params);

    // Log the closure coordinates.
    // ### Examples
    // ```
    // #[loggable(skip_closure_coords)] // Skip the closure coordinates in the internals of `f()`.
    // fn f() {
    //      #[loggable(log_closure_coords)] // Log the closure coordinates in the internals of `g()`.
    //      fn g() {
    //          Some(4).map(
    //              |x| x + 1); // Logs "closure{4,10:4,18}() {}".
    //      }
    //      Some(4).map(
    //          |x| x + 1); // Logs "closure{..}() {}".
    // }
    // ```
    syn::custom_keyword!(log_closure_coords);

    // Skip the closure coordiantes when logging.
    // ### Examples
    // ```
    // #[loggable]
    // fn f() {
    //      Some(4).map(
    //          |x| x + 1); // Logs "  f()::closure{4,10:4,18}() {}".
    // }
    // #[loggable(skip_closure_coords)]
    // fn g() {
    //      Some(4).map(
    //          |x| x + 1); // Logs "  g()::closure{..}() {}". The fragment `{..}` delimits a closure from a function named `closure`.
    // }
    // ```
    syn::custom_keyword!(skip_closure_coords);
}

struct FclQSelf {
    // <T as U::V>
    ty: Box<syn::Type>,
    path: syn::Path,
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
impl syn::parse::Parse for FclQSelf {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // <T as U::V>
        input.parse::<syn::Token![<]>()?;
        let ty = input.parse()?;
        input.parse::<syn::Token![as]>()?;
        let path = input.parse()?;
        input.parse::<syn::Token![>]>()?;
        Ok(Self { ty, path })
    }
}

enum LogPrefix {
    QSelf(FclQSelf),
    Path(syn::Path),
    // Empty    // `#[loggable(prefix=)]` (clearing the prefix).
}
struct QSelfOrPath(Option<LogPrefix>);

impl syn::parse::Parse for QSelfOrPath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = Self(None); // TODO: Consider `Self(Some(LogPrefix::Empty))`.
        if input.is_empty() {
            Ok(result)
        } else {
            let lookahead = input.lookahead1();
            // if lookahead.peek(syn::Token!["]) {
            // }
            if lookahead.peek(syn::Token![<]) {
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
    // Others, e.g. `Shallow`, // `#[loggable(shallow_params)]` (log param constructs _non-recursively_, i.e. skip (with `..`) the nested structs)
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
    params_logging: ParamsLogging,
    /// Whether to log the closure coordinates. Log (`true`) by default.
    /// ### Examples
    /// ```ignore
    /// #[loggable(skip_closure_coords)] // Skip the closure coordinates in the internals of `f()`.
    /// fn f() {
    ///      #[loggable(log_closure_coords)] // Log the closure coordinates in the internals of `g()`.
    ///      fn g() {
    ///          Some(4).map(
    ///              |x| x + 1); // Logs "closure{4,10:4,18}() {}".
    ///      }
    ///      Some(4).map(
    ///          |x| x + 1); // Logs "closure{..}() {}".
    /// }
    /// ```
    ///
    /// <br>
    ///
    /// ```ignore
    /// #[loggable]
    /// fn f() {
    ///      Some(4).map(
    ///          |x| x + 1); // Logs "  f()::closure{4,10:4,18}() {}".
    /// }
    /// #[loggable(skip_closure_coords)]
    /// fn g() {
    ///      Some(4).map(
    ///          |x| x + 1); // Logs "  g()::closure{..}() {}".
    ///          // The fragment `{..}` delimits a closure from a function named `closure`.
    /// }
    /// ```
    log_closure_coords: bool,
}
impl syn::parse::Parse for AttrArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attr_args = AttrArgs {
            prefix: quote! {},
            params_logging: ParamsLogging::Log,
            log_closure_coords: true,
        };
        loop {
            if input.is_empty() {
                break;
            }
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Token![,]) {
                // Skip any sequence of commas before, among, and after the attr args.
                input.parse::<syn::Token![,]>()?;
                continue;
            } else if lookahead.peek(kw::prefix) {
                input.parse::<kw::prefix>()?;
                input.parse::<syn::Token![=]>()?;
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
            } else if lookahead.peek(kw::skip_closure_coords) {
                input.parse::<kw::skip_closure_coords>()?;
                attr_args.log_closure_coords = false;
            } else if lookahead.peek(kw::log_closure_coords) {
                input.parse::<kw::log_closure_coords>()?;
                attr_args.log_closure_coords = true;
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
