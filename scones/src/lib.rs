use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    braced, parenthesized, parse_quote, Attribute, Error, Expr, Fields, Ident, ItemStruct, Path,
    Token, Type, Visibility,
};

#[derive(Clone)]
struct FieldInfo<'a> {
    ident: Ident,
    ty: &'a Type,
    custom_init: HashMap<String, TokenStream2>,
    default_init: Option<TokenStream2>,
}

enum ConstructorParam {
    /// A parameter which directly corresponds to a specific field.
    Field(Ident),
    /// A parameter which is custom and will be used to initialize other fields.
    Custom(Ident, Type),
    /// A stand-in for any Field parameters not explicitly specified.
    Ellipses,
}

impl Parse for ConstructorParam {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        if input.peek(Token![.]) && input.peek2(Token![.]) {
            let _: Token![.] = input.parse()?;
            let _: Token![.] = input.parse()?;
            Ok(Self::Ellipses)
        } else {
            let name: Ident = input.parse()?;
            if input.peek(Token![:]) {
                let _: Token![:] = input.parse()?;
                let ty: Type = input.parse()?;
                Ok(Self::Custom(name, ty))
            } else {
                Ok(Self::Field(name))
            }
        }
    }
}

struct ConstructorInfo {
    vis: Visibility,
    name: Ident,
    params: Vec<ConstructorParam>,
}

impl Parse for ConstructorInfo {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        // An empty input is also a visibility.
        let mut vis: Visibility = input.parse().unwrap();
        let (name, params): (Ident, _) = if input.peek(Token![fn]) {
            let _: Token![fn] = input.parse()?;
            let name: Ident = input.parse()?;
            let params = if input.is_empty() {
                vec![ConstructorParam::Ellipses]
            } else {
                let content;
                parenthesized!(content in input);
                let param_list = content.parse_terminated::<_, Comma>(ConstructorParam::parse)?;
                param_list.into_iter().collect()
            };
            (name, params)
        } else {
            // If they didn't even write "fn nam blah blah" then assume they want it publicly
            // visible.
            vis = parse_quote! { pub };
            (parse_quote!(new), vec![ConstructorParam::Ellipses])
        };
        Ok(Self { vis, name, params })
    }
}

fn make_constructor_args(
    constructor_name: &str,
    param_info: &[ConstructorParam],
    fields: &[FieldInfo],
) -> Result<TokenStream2, Error> {
    let mut param_impls = Vec::new();
    // Stores fields that must be in the parameters of the constructor but the user has not
    // yet explicitly specified where in the parameter list they should go.
    let mut remaining_fields: Vec<_> = fields
        .iter()
        .cloned()
        .filter(|e| !e.custom_init.contains_key(constructor_name) && e.default_init.is_none())
        .collect();
    // If we do not encounter an ellipses, then just insert the extra parameters at the end of the
    // signature.
    let mut remaining_fields_insertion_index = param_info.len();
    for param in param_info {
        match param {
            ConstructorParam::Field(field_name) => {
                let mut success = false;
                for (index, field) in remaining_fields.iter().enumerate() {
                    if &field.ident == field_name {
                        let field = remaining_fields.remove(index);
                        let name = field.ident;
                        let ty = &field.ty;
                        param_impls.push(quote! {
                            #name: #ty
                        });
                        success = true;
                        break;
                    }
                }
                if !success {
                    for field in fields {
                        if &field.ident == field_name {
                            let name = field.ident.clone();
                            let ty = &field.ty;
                            param_impls.push(quote! {
                                #name: #ty
                            });
                            success = true;
                            break;
                        }
                    }
                }
                if !success {
                    eprintln!("Missing field.");
                    return Err(Error::new_spanned(
                        field_name,
                        concat!(
                            "Could not find a field with this name ",
                            "(or it was used earlier in the constructor)"
                        ),
                    ));
                }
            }
            ConstructorParam::Custom(name, ty) => {
                param_impls.push(quote! {
                    #name: #ty
                });
            }
            ConstructorParam::Ellipses => {
                remaining_fields_insertion_index = param_impls.len();
            }
        }
    }
    for field in remaining_fields {
        let name = field.ident;
        let ty = &field.ty;
        param_impls.insert(
            remaining_fields_insertion_index,
            quote! {
                #name: #ty
            },
        );
        remaining_fields_insertion_index += 1;
    }
    Ok(quote! {
        #(#param_impls),*
    })
}

fn make_constructor_impl(
    info: ConstructorInfo,
    fields: &[FieldInfo],
) -> Result<TokenStream2, Error> {
    let vis = info.vis;
    let name = info.name;
    let name_str = name.to_string();
    let params = make_constructor_args(&name_str, &info.params[..], fields)?;
    let mut initializers = Vec::new();
    for field in fields {
        let ident = &field.ident;
        let init = field
            .custom_init
            .get(&name_str)
            .or(field.default_init.as_ref())
            .cloned()
            .unwrap_or(quote! { #ident });
        initializers.push(quote! {
            #ident: #init
        });
    }
    Ok(quote! {
        #vis fn #name (#params) -> Self {
            Self {
                #(#initializers),*
            }
        }
    })
}

struct ValueBody {
    expr: Expr,
    for_item: Option<Ident>,
}

impl Parse for ValueBody {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let interior;
        parenthesized!(interior in input);
        let expr: Expr = interior.parse()?;
        let for_item = if interior.is_empty() {
            None
        } else {
            let _: Token![for] = interior.parse()?;
            let name: Ident = interior.parse()?;
            Some(name)
        };
        Ok(Self { expr, for_item })
    }
}

fn path_equal(p1: &Path, p2: &Path) -> bool {
    if p1.leading_colon.is_some() != p2.leading_colon.is_some() {
        false
    } else if p1.segments.len() != p2.segments.len() {
        false
    } else {
        for (a, b) in p1.segments.iter().zip(p2.segments.iter()) {
            // We are not comparing any paths with arguments so not worrying about that.
            if a.ident.to_string() != b.ident.to_string() {
                return false;
            }
        }
        true
    }
}

struct GenerateItemsContent {
    args: TokenStream2,
}

impl Parse for GenerateItemsContent {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let interior;
        parenthesized!(interior in input);
        Ok(Self {
            args: interior.parse()?,
        })
    }
}

/// This can be invoked multiple times and it will produce a single #[make_constructor_internal]
/// invocation.
#[proc_macro_attribute]
pub fn make_constructor(input_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_attr2: TokenStream2 = input_attr.clone().into();
    // Check that the input is valid.
    let _: ConstructorInfo = syn::parse_macro_input!(input_attr);
    let macro_arg = quote! { constructor { #input_attr2 } };
    let mut struct_def: ItemStruct = syn::parse_macro_input!(item);
    let mut found = false;
    for attr in &mut struct_def.attrs {
        if path_equal(&attr.path, &parse_quote! { ::scones::generate_items__}) {
            let old_args: GenerateItemsContent = syn::parse2(attr.tokens.clone()).unwrap();
            let old_args = old_args.args;
            attr.tokens = quote! { ( #old_args #macro_arg ) };
            found = true;
            break;
        }
    }
    if !found {
        let attr_def = quote! {
            #[::scones::generate_items__(#macro_arg)]
        };
        struct_def
            .attrs
            .append(&mut (Attribute::parse_outer).parse2(attr_def).unwrap());
    }
    (quote! { #struct_def }).into()
}

struct GenerateItemsArgs {
    constructors: Vec<ConstructorInfo>,
}

impl Parse for GenerateItemsArgs {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut result = Self {
            constructors: Vec::new(),
        };
        while !input.is_empty() {
            let kind: Ident = input.parse()?;
            if kind == "constructor" {
                let content;
                braced!(content in input);
                result.constructors.push(content.parse()?);
            } else {
                unreachable!("Bad syntax generation");
            }
        }
        Ok(result)
    }
}

/// This is the actual macro that generates constructors. Use #{make_constructor} to invoke it.
#[proc_macro_attribute]
pub fn generate_items__(attr: TokenStream, item: TokenStream) -> TokenStream {
    let GenerateItemsArgs { constructors } = syn::parse_macro_input!(attr);
    let mut item_names: HashSet<String> = HashSet::new();
    for c in &constructors {
        item_names.insert(c.name.to_string());
    }

    let mut struct_def: ItemStruct = syn::parse_macro_input!(item);
    let struct_name = &struct_def.ident;
    let fields = if let Fields::Named(fields) = &mut struct_def.fields {
        &mut fields.named
    } else {
        return Error::new_spanned(
            &struct_def,
            "make_constructor currently only works on structs with named fields.",
        )
        .to_compile_error()
        .into();
    };
    let mut field_infos = Vec::new();
    for field in fields {
        let ident = field.ident.clone().unwrap();
        let mut condemned_indexes = Vec::new();
        let mut custom_init = HashMap::new();
        let mut default_init = None;
        for (index, attr) in field.attrs.iter().enumerate() {
            if attr.path.is_ident("value") {
                condemned_indexes.push(index);
                let tokens = attr.tokens.clone().into();
                let vb: ValueBody = syn::parse_macro_input!(tokens);
                let expr = vb.expr;
                let initializer = quote! { #expr };
                if let Some(for_item) = vb.for_item {
                    let item_name = for_item.to_string();
                    if !item_names.contains(&item_name) {
                        return Error::new_spanned(
                            for_item,
                            format!(
                                "The identifier \"{}\" does not refer to a constructor or builder.",
                                item_name
                            ),
                        )
                        .to_compile_error()
                        .into();
                    }
                    custom_init.insert(item_name, initializer);
                } else {
                    default_init = Some(initializer);
                }
            }
        }
        condemned_indexes.reverse();
        for index in condemned_indexes {
            field.attrs.remove(index);
        }
        field_infos.push(FieldInfo {
            ident,
            ty: &field.ty,
            custom_init,
            default_init,
        });
    }

    let mut constructor_defs = Vec::new();
    for cons in constructors {
        match make_constructor_impl(cons, &field_infos[..]) {
            Ok(def) => constructor_defs.push(def),
            Err(err) => return err.to_compile_error().into(),
        }
    }

    (quote! {
        #struct_def
        impl #struct_name {
            #(#constructor_defs)*
        }
    })
    .into()
}
