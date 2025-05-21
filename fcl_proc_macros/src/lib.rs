use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, spanned::Spanned, ExprClosure, ExprPath, ImplItemFn, ItemFn, Token};

// TODO: Consider moving to closure_logger and making it also a decl macro.
// Creates the `CallLogger` instance.
#[proc_macro]
pub fn call_logger(name: TokenStream) -> TokenStream {  // TODO: -> function_logger
    // Assert that the name is exactly one id (probably fully qualified like `MyStruct::method`). TODO: As opposed to what?
    let ts: proc_macro2::TokenStream = name.into();
    let func_name = ts.to_string(); // TODO: Should be something stringifyable of `syn`'s type.
    // TODO: Consider
    // * let func_name: ? = syn::parse(name);
    // * let func_name = syn::parse_macro_input!(name as String);
    quote! {
        use fcl::call_log_infra::CALL_LOG_INFRA;    // TODO: Consider moving to top of the file as a searate macro call.
        let mut _logger = None;
        CALL_LOG_INFRA.with(|infra| {
            if infra.borrow_mut().is_on() {
                _logger = Some(CallLogger::new(#func_name))
            }
        })
    }
    .into()
}

// TODO: _attr_args -> attr_args, _attributed_item -> attributed_item
#[proc_macro_attribute]
pub fn loggable(_attr_args: TokenStream, _attributed_item: TokenStream) -> TokenStream {
    // println!("{}", _attr_args);
    let attr_args = parse_attr_args(&_attr_args);

    // TODO: 
    // * Both assoc functions (ImplItemFn) and free-standing functions (ItemFn) are parsed the same way (by the one tried first).
    // * Both have generics (in my code missing for ImplItemFn).
    // Resolve:
    // * Either add generics to ImplItemFn,
    // * or figure out the differnece exactly and use the correct one in each case.
    if let Ok(func) = syn::parse::<ItemFn>(_attributed_item.clone()) {
        // A free-standing function.
        quote_as_itemfn(func, &attr_args)
    } else if let Ok(assoc_func) = syn::parse::<ImplItemFn>(_attributed_item.clone()) {
        quote_as_implitemfn(assoc_func, &attr_args)
    } else { 
    // // TODO: Make sure that both assoc functions and free-standing functions are not parsed the same way.
    // // WARNING: The order between `parse::<ImplItemFn>` and `parse::<ItemFn>` matters!
    // if let Ok(assoc_func) = syn::parse::<ImplItemFn>(_attributed_item.clone()) {
    //     quote_as_implitemfn(assoc_func, &attr_args)
    // } else if let Ok(func) = syn::parse::<ItemFn>(_attributed_item.clone()) {
    //     // A free-standing function.
    //     quote_as_itemfn(func, &attr_args)
    // } else { 
        // Handling closure differently because of an optional trailing comma, 
        // when closure is the last argument of a function.
        // This handling may erroneously consume comma if closure is NOT the last argument (TODO: Test).
        let closure_w_opt_comma = parse_macro_input!(_attributed_item as ExprClosureWOptComma);
        // TODO: What about parsing failure? (not a closure)
        let result = quote_as_closure(closure_w_opt_comma.closure, &attr_args);
        result

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
    //             //call_logger!(failed); // The `CallLogger` instance.
    //         }
    //     }
    //     .into()
    }

    // struct _AssocFunMethod { // Also includes TraitMethod
    //     // meta: (?)*
    //     // vis: Option<?>
    //     // name: String??
    //     // generics: ?
    //     // self: {enum|String|?}?       // &, &mut, self, ?
    //     // params: Vec<(String, Option<?>)>
    //     // params_optional_trailing_comma: bool,
    //     // ret_type: ?
    //     // body: ?
    // }
}

/*
// Creates the `ClosureLogger` instance.
#[proc_macro]
pub fn closure_logger(name: TokenStream) -> TokenStream {
    let ts: proc_macro2::TokenStream = name.into();
    let func_name = ts.to_string(); // TODO: Should be something stringifyable of `syn`'s type.
    // TODO: Consider
    // * let func_name: ? = syn::parse(name);
    // * let func_name = syn::parse_macro_input!(name as String);
    quote! {
        use fcl::call_log_infra::CALL_LOG_INFRA;    // TODO: Consider moving to top of the file as a searate macro call.
        let mut _l = None;
        CALL_LOG_INFRA.with(|infra| {
            if infra.borrow_mut().is_on() {
                _l = Some(ClosureLogger::new(#func_name))
            }
        })
    }
    .into()
}

*/

// TODO: -> quote_as_function
fn quote_as_itemfn(func: ItemFn, attr_args: &Option<AttrArgs>) -> TokenStream {
    let attrs = func.attrs;
    let vis = func.vis;
    let signature = func.sig;
    let func_name = if let Some(attr_args) = attr_args { 
        // "MyTrait::my_func". 
        // TODO: Review if this block is still applicable for `quote_as_itemfn()`. 
        // In other words, considere leaving such a block 
        // for `quote_as_implitemfn()` only.
        let path = attr_args.name.path.clone();
        // let path = attr_args.name.to_token_stream();
        // let path = attr_args.name.clone();

        // let my_ident = quote::format_ident!("My{}{}", path.segments.first().unwrap(), "IsCool");

        let ret = quote! { #path }; // .clone()
        // println!("fffffffffff {ret}");
        ret
    } else {
        // "my_func"
        let id = signature.ident.clone();
        quote! { #id }
        // signature.ident.to_string()  //clone() 
    };
    let generics = signature.generics.clone();

    // let func_name = if let Some(attr_args) = attr_args { 
    //     let path = attr_args.name.clone();
    //     quote! { #path }// .clone()
    // } else { 
    //     let id = signature.ident.clone();
    //     quote! { #id }
    //     // signature.ident.to_string()  //clone() 
    // };    
    // let func_name = if let Some(attr_args) = attr_args { 
    //     attr_args.name.clone()
    // } else { 
    //     signature.ident.clone()
    //     // signature.ident.to_string()  //clone() 
    // };    

    // let func_name = if let Some(attr_args) = attr_args { 
    //     attr_args.name.clone()
    // } else { 
    //     signature.ident.to_string()  //clone() 
    //     // signature.ident.clone()
    // };    

    // let func_name = signature.ident.clone();

    let block = func.block; // TODO: Consider `block` -> `body`.
    let output = quote! {
        #(#attrs)*
        #vis #signature {
            call_logger!(#func_name #generics); // The `CallLogger` instance. // TODO: Consider: `#func_name #generics` -> `#func_name#generics` (remove space)
            #block
        }
        // $( #[$meta] )*
        // $vis fn $name ( $( $arg_name : $arg_ty ),* ) $( -> $ret_ty )? {
        //     call_logger!($name); // The `CallLogger` instance.
        //     // TODO: Consider `call_logger!("$name");` // Quoted arg.
        //     $($tt)*
        // }
    };
    output.into()
}

// TODO: -> quote_as_assoc_function
fn quote_as_implitemfn(func: ImplItemFn, attr_args: &Option<AttrArgs>) -> TokenStream {
    let attrs = func.attrs;
    let vis = func.vis;
    let defaultness = func.defaultness;
    let signature = func.sig;
    let func_name = if let Some(attr_args) = attr_args { 
        // "MyTrait::my_func"
        let path = attr_args.name.path.clone();
        // let path = attr_args.name.to_token_stream();
        // let path = attr_args.name.clone();
        let ret = quote! { #path }; // .clone()
        // println!("iiiiiiiiiiiii {ret}");
        ret
    } else { 
        // "my_func"
        let id = signature.ident.clone();
        quote! { #id }
        // signature.ident.to_string()  //clone() 
    };    
    // let func_name = signature.ident.clone();
    let block = func.block; // TODO: Consider block -> body.
    let output = quote! {
        #(#attrs)*
        #vis #defaultness #signature {
            call_logger!(#func_name); // The `CallLogger` instance. TODO: What about `#generics`?
            #block
        }
    };
    output.into()
}


fn quote_as_closure(closure: ExprClosure, _attr_args: &Option<AttrArgs>) -> TokenStream {
    // TODO: _attr_args
// fn quote_as_closure(closure: ExprClosureComma) -> TokenStream {    
    // pub attrs        : Vec<Attribute>,
    // pub lifetimes    : Option<BoundLifetimes>,
    // pub constness    : Option<Const>,
    // pub movability   : Option<Static>,
    // pub asyncness    : Option<Async>,
    // pub capture      : Option<Move>,
    // pub or1_token    : Or,
    // pub inputs       : Punctuated<Pat, Comma>,
    // pub or2_token    : Or,
    // pub output       : ReturnType,
    // pub body         : Box<Expr>,

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
        // #[rustfmt::skip]
        #lifetimes #constness #movability #asyncness #capture 
        #or1_token #inputs #or2_token #output 
        {
            // call_logger!(closure); 
            closure_logger!(#start_line, #start_col, #end_line, #end_col); // The `CallLogger` instance. 
            // Closure has no name. The name will be replaced with "enclosing()::closure_line_col()". // TODO: Update this naming comment.
            // TODO: Is `#generics` applicable to closures?

            // println!("start: {{ {}, {} }}.", #start_line, #start_col);
            // println!("end: {{ {}, {} }}.", #end_line, #end_col);
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

struct AttrArgs {
    name: ExprPath //Ident // String
}

fn parse_attr_args(attr_args: &TokenStream) -> Option<AttrArgs> {
    if !attr_args.is_empty() {
        
        if let Ok(id) = syn::parse::<ExprPath>(attr_args.clone()) {
        // if let Ok(id) = syn::parse::<Ident>(attr_args.clone()) {
            // if let Ok(literal) = syn::parse::<ExprLit>(attr_args.clone()) {
                // if let Lit::Str(str) = id.lit {
                    return Some(AttrArgs {
                        name: id
                        // name: str.value()
                    })
                // }
            }
    }
    None
}

