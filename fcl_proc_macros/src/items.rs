use quote::quote;
use crate::{AttrArgs, IsTraverseStopper, LoggableAttrInfo, ParamsLogging, quote_as_expr, traversed_block_from_sig};

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

fn get_loggable_attr_params_meta_tokens(user_provided_attr_info: &LoggableAttrInfo, enclosing_item_attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    let mut updated_tokens = quote!{};
    let LoggableAttrInfo {
        prefix: user_provided_prefix,
        params_logging: user_provided_params_logging,
        log_closure_coords: user_provided_log_closure_coords,
    } = user_provided_attr_info;

    let new_prefix = user_provided_prefix.as_ref().unwrap_or(&enclosing_item_attr_args.prefix/* .clone()*/);
    // println!("prefix: {:?}", prefix);
    updated_tokens = quote!{ prefix = #new_prefix, };
    // println!("updated_tokens: {:?}", updated_tokens);
    // if let Some(specified_prefix) = prefix {
    //     updated_tokens = quote!{ prefix = #specified_prefix, };
    // } else {
    //     let inherited_prefix = &attr_args.prefix;
    //     updated_tokens = quote!{ prefix = #inherited_prefix, };
    // }

    let new_params_logging = user_provided_params_logging.unwrap_or(enclosing_item_attr_args.params_logging);
    match new_params_logging {
        ParamsLogging::Log => updated_tokens = quote!{ #updated_tokens log_params, },
        ParamsLogging::Skip => updated_tokens = quote!{ #updated_tokens skip_params, },
        // Any new ones require attention.
    }
    // println!("log_closure_coords: {:?}", log_closure_coords);
    let new_log_closure_coords = user_provided_log_closure_coords.unwrap_or(enclosing_item_attr_args.log_closure_coords);
    if new_log_closure_coords {
        updated_tokens = quote!{ #updated_tokens log_closure_coords, }
    } else {
        updated_tokens = quote!{ #updated_tokens skip_closure_coords, }
    }
    // println!("updated_tokens: {:?}", updated_tokens);
    updated_tokens
}

fn handle_loggable_attr_params(attr: &syn::Attribute, has_loggable: &mut bool, enclosing_item_attr_args: &AttrArgs, new_attrs: &mut Vec<syn::Attribute>) {
    if let Some(user_provided_attr_info) = attr.get_loggable_attr_info() {
        *has_loggable = true;
        let new_attr = syn::Attribute {
            pound_token: attr.pound_token,
            style: attr.style,
            bracket_token: attr.bracket_token,
            meta: match &attr.meta {
                syn::Meta::List(metalist) =>
                    syn::Meta::List(syn::MetaList {
                        path: metalist.path.clone(),
                        delimiter: metalist.delimiter.clone(),
                        tokens: {
                            get_loggable_attr_params_meta_tokens(&user_provided_attr_info, enclosing_item_attr_args)
                        }
                    }),
                // Path(Path),
                // NameValue(MetaNameValue),
                _ => attr.meta.clone(),
            },
        };
        // println!("new_attr: {:?}", new_attr.meta.);
        new_attrs.push(new_attr);
    } else {
        new_attrs.push(attr.clone());
    }
}

fn quote_as_item_fn(
    item_fn: &syn::ItemFn,
    enclosing_item_attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ItemFn {
        attrs, //: Vec<Attribute>,
        vis,   //: Visibility,
        sig,   //: Signature,
        block, //: Box<Block>,
    } = item_fn;
    // println!("{:?} {{", sig.ident);

    let mut new_attrs = vec![];
    let mut has_loggable = false;

    // println!("attrs.len(): {}", attrs.len());
    for attr in attrs {
        // match &attr.meta {
        //     Meta::Path(path) => println!("Path: {:?}", path.get_ident()),
        //     Meta::List(metalist) => println!("Meta::List Path{:?}", metalist.path.get_ident()),
        //     Meta::NameValue(meta_name_value) => println!("NameValue Path: {:?}", meta_name_value.path.get_ident()),
        // }
        // println!("attr: {:?}", attr.meta);

        if attr.is_non_loggable() {
        // if attr.is_traverse_stopper() {
            // println!("}} {:?} // non_loggable", sig.ident);

            return quote! { #item_fn };
        }
        handle_loggable_attr_params(attr, &mut has_loggable, enclosing_item_attr_args, &mut new_attrs);
    }

    let block = if has_loggable {
        // println!("not traversing");

        // After updating/adding the params of/to #[loggable(<params>)]
        // leave the fn body uninstrumented, so that a separate #[loggable(<params>)] macro invocation 
        // will instrument the body.
        quote!{ #block }
        // block
    } else {
        // println!("traversing");
        traversed_block_from_sig(block, sig, enclosing_item_attr_args)
    };

    // println!("{:?} }} // end", sig.ident);

    // // let attrs = if new_attrs.is_empty() { attrs } else { &new_attrs };
    // let block = traversed_block_from_sig(block, sig, attr_args);
    let ret_val = quote! { #(#new_attrs)* #vis #sig #block };
    // println!("quote_as_item_fn()::ret_val: {}", ret_val);
    ret_val
    // quote! { #(#new_attrs)* #vis #sig #block }
    // quote! { #(#attrs)* #vis #sig #block }
}

// // Likely not applicable for instrumenting the run time functions and
// // closures (as opposed to compile time const functions and closures).
// fn quote_as_item_foreign_mod(item_foreign_mod: &ItemForeignMod, _attr_args: &AttrArgs) -> proc_macro2::TokenStream {
//     // let ItemForeignMod {} = item_foreign_mod;
//     quote!{ #item_foreign_mod }
// }
fn quote_as_impl_item_fn(
    impl_item_fn: &syn::ImplItemFn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ImplItemFn {
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
    impl_item: &syn::ImplItem,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    match impl_item {
        syn::ImplItem::Fn(impl_item_fn) => quote_as_impl_item_fn(impl_item_fn, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::ImplItem::Const(impl_item_const) => quote_as_impl_item_const(impl_item_const, attr_args),
        // syn::ImplItem::Type(impl_item_type) => quote_as_impl_item_type(impl_item_type, attr_args),
        // syn::ImplItem::Macro(impl_item_macro) => quote_as_impl_item_macro(impl_item_macro, attr_args),
        // syn::ImplItem::Verbatim(token_stream) => quote_as_token_stream(token_stream, attr_args),
        other => quote! { #other },
    }
}
fn quote_as_item_impl(
    item_impl: &syn::ItemImpl,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ItemImpl {
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

    let prefix_extender = match trait_.as_ref() {
        // trait impl
        Some((_opt_not, path, _for_token)) => quote! { <#self_ty as #path> },
        // struct impl
        None => quote! { #self_ty }
    };
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
                quote! { #prefix_extender }
            } else {
                let prefix = &attr_args.prefix;
                quote! { #prefix::#prefix_extender }
            },
            ..*attr_args
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
    item_mod: &syn::ItemMod,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ItemMod {
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
        ..*attr_args
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
    item_static: &syn::ItemStatic,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ItemStatic {
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
    trait_item_const: &syn::TraitItemConst,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::TraitItemConst {
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
    trait_item_fn: &syn::TraitItemFn,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::TraitItemFn {
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
    item: &syn::TraitItem,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    match item {
        syn::TraitItem::Const(trait_item_const) => quote_as_trait_item_const(trait_item_const, attr_args),
        syn::TraitItem::Fn(trait_item_fn) => quote_as_trait_item_fn(trait_item_fn, attr_args),

        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::TraitItem::Type(trait_item_type) => quote_as_trait_item_type(trait_item_type, attr_args),
        // syn::TraitItem::Macro(trait_item_macro) => quote_as_trait_item_macro(trait_item_macro, attr_args),

        // syn::TraitItem::Verbatim(token_stream) => quote_as_token_stream(token_stream, attr_args),
        other => quote! { #other },
    }
}
fn quote_as_item_trait(
    item_trait: &syn::ItemTrait,
    attr_args: &AttrArgs,
) -> proc_macro2::TokenStream {
    let syn::ItemTrait {
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

    // NOTE: Future: `restriction`. Unused, but reserved for RFC 3323 restrictions.

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

    // NOTE: The traits are defined at compile time 
    // when the actual generic arguments are not known yet 
    // (and whether the trait will be used at all).
    // That's why we cannot expand the traits' `#generics` when extending the prefix.
    let items = {
        let attr_args = AttrArgs { prefix:  
            if attr_args.prefix.is_empty() {
                quote! { #ident #generics }
            } else {
                let prefix = &attr_args.prefix;
                quote! { #prefix::#ident #generics }
            },
            ..*attr_args
        };
        let mut traversed_items = quote! {};
        for item in items {
            let traversed_item = quote_as_trait_item(item, &attr_args);
            traversed_items = quote! { #traversed_items #traversed_item };
        }
        traversed_items
    };
    quote! { #(#attrs)* #vis #unsafety #auto_token // #restriction
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

pub fn quote_as_item(item: &syn::Item, attr_args: &AttrArgs) -> proc_macro2::TokenStream {
    match item {
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::Item::Const(item_const) => quote_as_item_const(item_const, attr_args),
        // syn::Item::Enum(item_enum) => quote_as_item_enum(item_enum, attr_args),
        // syn::Item::ExternCrate(item_extern_crate) => quote_as_item_extern_crate(item_extern_crate, attr_args),
        syn::Item::Fn(item_fn) => quote_as_item_fn(item_fn, attr_args),
        // syn::Item::ForeignMod(item_foreign_mod) => quote_as_item_foreign_mod(item_foreign_mod, attr_args),
        syn::Item::Impl(item_impl) => quote_as_item_impl(item_impl, attr_args),
        // syn::Item::Macro(item_macro) => quote_as_item_macro(item_macro, attr_args),
        syn::Item::Mod(item_mod) => quote_as_item_mod(item_mod, attr_args),
        syn::Item::Static(item_static) => quote_as_item_static(item_static, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::Item::Struct(item_struct) => quote_as_item_struct(item_struct, attr_args),
        syn::Item::Trait(item_trait) => quote_as_item_trait(item_trait, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::Item::TraitAlias(item_trait_alias) => quote_as_item_trait_alias(item_trait_alias, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::Item::Type(item_type) => quote_as_item_type(item_type, attr_args),
        // // Likely not applicable for instrumenting the run time functions and
        // // closures (as opposed to compile time const functions and closures).
        // syn::Item::Union(item_union) => quote_as_item_union(item_union, attr_args),
        // syn::Item::Use(item_use) => quote_as_item_use(item_use, attr_args),
        // syn::Item::Verbatim(token_stream) => quote_as_token_stream(token_stream, attr_args)
        other => quote! { #other }, // syn::Item::{Const,Enum,Union,Verbatim}
    }
}