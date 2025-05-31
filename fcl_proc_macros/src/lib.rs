use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, parse_quote, spanned::Spanned, Attribute, ExprClosure, ExprPath, ImplItemFn, Item, ItemFn, ItemImpl, ItemMod, Token};

// TODO: Likely out-of-date since doesn't handle generics properly.
// TODO: Consider moving to closure_logger and making it also a decl macro.
// Creates the `FunctionLogger` instance.
#[proc_macro]
pub fn function_logger(name: TokenStream) -> TokenStream {
    // Assert that the name is exactly one id (probably fully qualified like `MyStruct::method`). TODO: As opposed to what?
    let ts: proc_macro2::TokenStream = name.into();
    let func_name = ts.to_string(); // TODO: Should be something stringifyable of `syn`'s type.
    // TODO: Consider
    // * let func_name: ? = syn::parse(name);
    // * let func_name = syn::parse_macro_input!(name as String);
    quote! {
        let mut _logger = None;
        fcl::call_log_infra::THREAD_LOGGER.with(|logger| {
            if logger.borrow_mut().logging_is_on() {
                _logger = Some(FunctionLogger::new(#func_name))
            }
        });
    }
    .into()
}

#[proc_macro_attribute]
pub fn loggable(attr_args: TokenStream, attributed_item: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(attr_args as AttrArgs); // Handles the compilation errors appropriately.
/* 
    #[proc_macro_attribute]
    pub fn my_attr(args: TokenStream, input: TokenStream) -> TokenStream {
        let args = parse_macro_input!(args as MyAttrArgs);
*/    
    // let attr_args = parse_attr_args(&attr_args);

    // TODO: 
    // * Both assoc functions (ImplItemFn) and free-standing functions (ItemFn) are currently parsed 
    //   the same way (by the one tried first).
    // * Both have generics (in my code missing for ImplItemFn).
    // Resolve:
    // * Either add generics to ImplItemFn,
    // * or figure out the differnece exactly and use the correct one in each case.
    // TODO:
    // #[loggable(prefix=Parent)]
    // impl ...
    if let Ok(module_block) = syn::parse::<ItemMod>(attributed_item.clone()) {
        quote_as_module(module_block, &attr_args)
    } else if let Ok(impl_block) = syn::parse::<ItemImpl>(attributed_item.clone()) {
        quote_as_impl(impl_block, &attr_args)
    } else if let Ok(func) = syn::parse::<ItemFn>(attributed_item.clone()) {
        // A free-standing function.
        quote_as_function(func, &attr_args)
    } else if let Ok(assoc_func) = syn::parse::<ImplItemFn>(attributed_item.clone()) {
        quote_as_associated_function(assoc_func, &attr_args)

    // TODO: Review the closure parsing below such that after the closure 
    // if nothing is parsed/recognized successfully then just forward the input to the output.
    // E.g. if `#[loggable] impl` has non-function items, e.g. `type ...` (that get recursively marked as `#[loggable]`), 
    // then those items just need to be forwarded from input to output (with `#[loggable]` removed).

    } else if let Ok(closure_w_opt_comma) = 
        syn::parse::<ExprClosureWOptComma>(attributed_item.clone()) 
    {
        let result = quote_as_closure(closure_w_opt_comma.closure, &attr_args);
        result
    } else {
        // TODO: Compiler error instead of forwarding.
        attributed_item
    }
    // } else {
    //     // Handling closure differently because of an optional trailing comma, 
    //     // when closure is the last argument of a function.
    //     // This handling may erroneously consume comma if closure is NOT the last argument (TODO: Test).
    //     let closure_w_opt_comma = 
    //         parse_macro_input!(attributed_item as ExprClosureWOptComma); // Handles the compilation errors appropriately.
    //     // TODO: What about parsing failure? (not a closure) Consider TODO below.
    //     let result = quote_as_closure(closure_w_opt_comma.closure, &attr_args);
    //     result
    // }    

    // } else if let Ok(closure) = syn::parse::<ExprClosure>(&_attributed_item) {
    //     let result = quote_as_closure(closure);
    //     // TODO: Optional trailing comma after the closure.
    //     // syn::parse::<Option<Token![,]>>(_attributed_item);
    //
    //     result

    // } else {
    //     // TODO: Compiler error:
    //     // Failed to parse as a callable (function //, associated function, closure,
    //     quote! {
    //         fn failed () {
    //             //function_logger!(failed); // The `FunctionLogger` instance.
    //         }
    //     }
    //     .into()
    // }
}

/*
// Creates the `ClosureLogger` instance.
#[proc_macro]
pub fn closure_logger(name: TokenStream) -> TokenStream {
    let ts: proc_macro2::TokenStream = name.into();
    let func_name = ts.to_string(); // TODO: Should be something stringifyable of `syn`'s type.
    quote! {
        let mut _l = None;
        fcl::call_log_infra::CALL_LOG_INFRA.with(|infra| {
            if infra.borrow_mut().logging_is_on() {
                _l = Some(ClosureLogger::new(#func_name))
            }
        })
    }
    .into()
}

*/

#[rustfmt::skip]
fn quote_as_module(module_block: ItemMod, attr_args: &AttrArgs) -> TokenStream {
    let ItemMod {
        attrs, // : Vec<Attribute>,
        vis, // : Visibility,
        unsafety, // : Option<Unsafe>,
        mod_token, // : Mod,
        ident, // : Ident,
        content, // : Option<(Brace, Vec<Item>)>,
        semi, // : Option<Semi>,        
    } = module_block;

    let mut output = quote! {
        #(#attrs)* // : Vec<Attribute>,
        #vis // : Visibility,
        #unsafety // : Option<Unsafe>,
        #mod_token // : Mod,
        #ident // : Ident,
    };

    if let Some((_, item_vec)) = content {
        let mut prefix = quote! { #ident };
        if let AttrArgs::Prefix { path, .. } = attr_args {
            prefix = quote!{ #path::#prefix };
        }
        let loggable_attr: Attribute = parse_quote! {
            #[fcl_proc_macros::loggable(prefix=#prefix)]
        };

        let mut content = quote! {};
        for item in &item_vec {
            match item {
                Item::Fn(_) | 
                Item::Impl(_) | 
                Item::Mod(_)  => content = quote! { #content #loggable_attr #item },
                _             => content = quote! { #content                #item },
            }
        }
        output = quote! {
            #output {
                #content
            }
        }
        // output = quote! {
        //     #output {
        //         #(#loggable_attr #item_vec)*
        //     }
        // }
    }
    output = quote! {
        #output
        #semi
    };
    output.into()
}

fn quote_as_impl(impl_block: ItemImpl, _attr_args: &AttrArgs) -> TokenStream {
    let ItemImpl {
        attrs,
        defaultness,
        unsafety,
        impl_token,
        generics,
        trait_,
        self_ty,
        items,
        .. // brace_token,
    } = impl_block;
    let loggable_attr: Attribute = parse_quote! {
        #[fcl_proc_macros::loggable(prefix=#self_ty)]
    };

    let mut output = quote! {
        #(#attrs)*
        #defaultness
        #unsafety
        #impl_token
        #generics
    };
    // #trait_
    if let Some((exclamation, path, for_token)) = trait_ {
        output = quote!{
            #output
            #exclamation #path #for_token
        };
    }
    output = quote!{
        #output
        #self_ty
        // #brace_token
        {
            #(#loggable_attr #items)*
            // #(#items)*
        }
    };

    output.into()
}

fn quote_as_function(func: ItemFn, attr_args: &AttrArgs /*&Option<AttrArgs>*/) -> TokenStream {
    let attrs = func.attrs;
    let vis = func.vis;
    let signature = func.sig;
    let func_name = match attr_args {
        AttrArgs::Name { path, .. } => quote!{ #path },
        AttrArgs::Prefix { path, .. } => {
            let id = signature.ident.clone();
            quote!{ #path::#id }
        }
        AttrArgs::None => {
            let id = signature.ident.clone();
            quote!{ #id }
        }
    };
    // let func_name = if let Some(attr_args) = attr_args { 
    //     // "MyTrait::my_func". 
    //     let path = attr_args.name.path.clone();
    //     let ret = quote! { #path }; // .clone()
    //     ret
    // } else {
    //     // "my_func"
    //     let id = signature.ident.clone();
    //     quote! { #id }
    // };
    let generics = signature.generics.clone();
    let generics_params_iter = generics.type_params();
    let empty_generic_params = generics.params.is_empty();

    let body = func.block;
    let output = quote! {
        #(#attrs)*
        #vis #signature {
            let mut generic_func_name = String::with_capacity(64);
            generic_func_name.push_str(stringify!(#func_name));
            if !#empty_generic_params {
                generic_func_name.push_str("<");
                let param_vec: Vec<&'static str> = vec![#(std::any::type_name::< #generics_params_iter >(),)*];
                for type_iter in param_vec {
                    generic_func_name.push_str(type_iter);
                    generic_func_name.push_str(",");
                }
                generic_func_name.push_str(">");
            }
            
            let mut _logger = None;
            fcl::call_log_infra::THREAD_LOGGER.with(|logger| {
                if logger.borrow_mut().logging_is_on() {
                    _logger = Some(fcl::FunctionLogger::new(&generic_func_name))
                    // _logger = Some(FunctionLogger::new(#func_name))
                }
            }); 

            // function_logger!(#func_name #generics); // The `FunctionLogger` instance.
            #body
        }
        // $( #[$meta] )*
        // $vis fn $name ( $( $arg_name : $arg_ty ),* ) $( -> $ret_ty )? {
        //     function_logger!($name); // The `FunctionLogger` instance.
        //     $($tt)*
        // }
    };
    output.into()
}

fn quote_as_associated_function(func: ImplItemFn, attr_args: &AttrArgs) -> TokenStream {
    let attrs = func.attrs;
    let vis = func.vis;
    let defaultness = func.defaultness;
    let signature = func.sig;
    // TODO: Dedup func_name for quote_as_associated_function() and quote_as_function().
    let func_name = match attr_args {
        AttrArgs::Name { path, .. } => quote!{ #path },
        AttrArgs::Prefix { path, .. } => {
            let id = signature.ident.clone();
            quote!{ #path::#id }
        }
        AttrArgs::None => {
            let id = signature.ident.clone();
            quote!{ #id }
        }
    };
    // let func_name = if let Some(attr_args) = attr_args { 
    //     // "MyTrait::my_func"
    //     let path = attr_args.name.path.clone();
    //     let ret = quote! { #path };
    //     ret
    // } else { 
    //     // "my_func"
    //     let id = signature.ident.clone();
    //     quote! { #id }
    // };    
    let body = func.block;
    let output = quote! {
        #(#attrs)*
        #vis #defaultness #signature {
            function_logger!(#func_name); // The `FunctionLogger` instance. TODO: What about `#generics`? Forgotten!
            #body
        }
    };
    output.into()
}


fn quote_as_closure(closure: ExprClosure, _attr_args: &AttrArgs) -> TokenStream {
    let (start_line, start_col) = {
        let proc_macro2::LineColumn{ line, column } = 
            proc_macro2::Span::call_site().start();
        (line, column + 1)
    };
    let (end_line, end_col) = {
        let proc_macro2::LineColumn{ line, column } = 
            // proc_macro2::Span::call_site().end();
            closure.body.span().end();
        (line, column)
    };
    let attrs   = closure.attrs;
    let lifetimes = closure.lifetimes;
    let constness = closure.constness;
    let movability = closure.movability;
    let asyncness = closure.asyncness;
    let capture = closure.capture;
    let or1_token = closure.or1_token;
    let inputs = closure.inputs;
    let or2_token = closure.or2_token;
    let output = closure.output;
    let body = closure.body;

    let output = quote! {
        #(#attrs)*
        #lifetimes #constness #movability #asyncness #capture 
        #or1_token #inputs #or2_token #output 
        {
            // The `ClosureLogger` instance:
            closure_logger!(#start_line, #start_col, #end_line, #end_col);
            #body              
        }
    };
    output.into()
}

/// Closure with optional trailing comma 
/// (when closure is the last argument of a function):
struct ExprClosureWOptComma {
    closure: ExprClosure,
    _optional_comma: Option<Token![,]>
}
impl Parse for ExprClosureWOptComma {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(ExprClosureWOptComma {
            closure: input.parse()?,
            _optional_comma: input.parse()?
        })
    }
}

mod kw {
    syn::custom_keyword!(name);
    syn::custom_keyword!(prefix);
}

enum AttrArgs {
    // TODO: Dedup or remove `eq_token` and `path`.
    Name{
        _name_token: kw::name,
        _eq_token: Token![=],
        path: ExprPath
    },
    Prefix{
        _prefix_token: kw::prefix,
        _eq_token: Token![=],
        path: ExprPath
    },
    None
}

impl Parse for AttrArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(AttrArgs::None)
        }
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::name) {
            Ok(AttrArgs::Name {
                _name_token: input.parse::<kw::name>()?,
                _eq_token: input.parse()?,
                path: input.parse()?,
            })
        } else if lookahead.peek(kw::prefix) {
            Ok(AttrArgs::Prefix {
                _prefix_token: input.parse::<kw::prefix>()?,
                _eq_token: input.parse()?,
                path: input.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

// struct ArgExpr {
//     name_or_prefix: Ident,
//     eq: Token![=],
//     path: ExprPath,
// }
// impl Parse for ArgExpr {
//     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
//         Ok(Self {
//             name_or_prefix: input.parse()?,
//             eq: input.parse()?,
//             path: input.parse()?,
//             // closure: input.parse()?,
//             // _optional_comma: input.parse()?
//         })
        
//     }
// }

// struct AttrArgs {
//     name: ExprPath
// }

// fn parse_attr_args(attr_args: &TokenStream) -> Option<syn::Result<AttrArgs>> {
//     if !attr_args.is_empty() {
//         return Some(syn::parse::<AttrArgs>(attr_args.clone()))
//         // match syn::parse::<AttrArgs>(attr_args.clone()) {
//         //     Ok(attr_args) => return Some(attr_args),
//         //     Err(_err) => {
//         //         // TODO: Log the attr arg error appropriately.
//         //         return None
//         //     }
//         // }
        
//         // if let Ok(ArgExpr { name_or_prefix, path, .. } ) = syn::parse::<ArgExpr>(attr_args.clone()) {
//         // }

//         // if let Ok(id) = syn::parse::<ExprPath>(attr_args.clone()) {
//         // // if let Ok(id) = syn::parse::<Ident>(attr_args.clone()) {
//         //     // if let Ok(literal) = syn::parse::<ExprLit>(attr_args.clone()) {
//         //         // if let Lit::Str(str) = id.lit {
//         //             return Some(AttrArgs {
//         //                 name: id
//         //                 // name: str.value()
//         //             })
//         //         // }
//         //     }
//     }
//     None
// }

