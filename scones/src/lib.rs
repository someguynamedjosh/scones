use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    braced, parenthesized, parse_quote, Error, Expr, Fields, Ident, ItemStruct, Token, Type,
    Visibility,
};

#[derive(Clone)]
struct FieldInfo<'a> {
    ident: &'a Option<Ident>,
    ty: &'a Type,
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
            Ok(Self::Ellipses)
        } else {
            let name: Ident = input.parse()?;
            if input.is_empty() {
                Ok(Self::Field(name))
            } else {
                let _: Token![:] = input.parse()?;
                let ty: Type = input.parse()?;
                Ok(Self::Custom(name, ty))
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
        let mut input = &*input;
        let fork = input.fork();
        let maybe_vis = &fork;
        let vis: Visibility = if let Ok(vis) = maybe_vis.parse() {
            input = maybe_vis;
            vis
        } else {
            parse_quote! { pub }
        };
        let (name, params): (Ident, _) = if input.is_empty() {
            (parse_quote!(new), vec![ConstructorParam::Ellipses])
        } else {
            let _: Token![fn] = input.parse()?;
            let name: Ident = input.parse()?;
            let params = if input.is_empty() {
                vec![ConstructorParam::Ellipses]
            } else {
                let content;
                braced!(content in input);
                let param_list = content.parse_terminated::<_, Comma>(ConstructorParam::parse)?;
                param_list.into_iter().collect()
            };
            (name, params)
        };
        Ok(Self { vis, name, params })
    }
}

fn make_constructor_args(param_info: &[ConstructorParam], fields: &[FieldInfo]) -> TokenStream2 {
    let mut param_impls = Vec::new();
    let mut remaining_fields: Vec<_> = fields.iter().map(|e| e.clone()).collect();
    // If we do not encounter an ellipses, then just insert the extra parameters at the end of the
    // signature.
    let mut remaining_fields_insertion_index = param_info.len();
    for param in param_info {
        match param {
            ConstructorParam::Field(field_name) => {
                let mut success = false;
                for (index, field) in remaining_fields.iter().enumerate() {
                    if field.ident.as_ref() == Some(field_name) {
                        let field = remaining_fields.remove(index);
                        let name = field.ident.as_ref().unwrap(); // Tuple structs not implemented.
                        let ty = &field.ty;
                        param_impls.push(quote! {
                            #name: #ty
                        });
                        success = true;
                        break;
                    }
                }
                if !success {
                    return Error::new_spanned(
                        field_name,
                        concat!(
                            "Could not find a field with this name ",
                            "(or it was used earlier in the constructor)"
                        ),
                    )
                    .to_compile_error();
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
        let name = field.ident.as_ref().unwrap(); // Tuple structs not implemented.
        let ty = &field.ty;
        param_impls.push(quote! {
            #name: #ty
        });
        remaining_fields_insertion_index += 1;
    }
    quote! {
        #(#param_impls),*
    }
}

fn make_constructor_impl(info: ConstructorInfo, fields: &[FieldInfo]) -> TokenStream2 {
    let vis = info.vis;
    let name = info.name;
    let params = make_constructor_args(&info.params[..], fields);
    quote! {
        #vis fn #name (#params) {

        }
    }
}

#[proc_macro_attribute]
pub fn make_constructor(attr: TokenStream, item: TokenStream) -> TokenStream {
    let constructor_info: ConstructorInfo = syn::parse_macro_input!(attr);
    let mut constructors = vec![constructor_info];

    let struct_def: ItemStruct = syn::parse_macro_input!(item);
    let struct_name = &struct_def.ident;
    let fields = if let Fields::Named(fields) = &struct_def.fields {
        &fields.named
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
        field_infos.push(FieldInfo {
            ident: &field.ident,
            ty: &field.ty,
        });
    }

    let mut constructor_defs = Vec::new();
    for cons in constructors {
        constructor_defs.push(make_constructor_impl(cons, &field_infos[..]));
    }

    (quote! { 
        #struct_def 
        impl #struct_name {
            #(#constructor_defs)* 
        }
    }).into()
}
