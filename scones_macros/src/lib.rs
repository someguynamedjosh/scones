use inflector::Inflector;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::token::{Comma, Paren};
use syn::{
    braced, parenthesized, parse_quote, Attribute, Error, Expr, Fields, GenericParam, Generics,
    Ident, ItemStruct, Path, Token, Type, Visibility,
};

#[derive(Clone)]
struct FieldInfo<'a> {
    ident: Ident,
    ty: &'a Type,
    custom_init: HashMap<String, TokenStream2>,
    default_init: Option<TokenStream2>,
}

enum ReturnSemantics {
    Selff,
    Result,
}

enum BuilderParam {
    Field {
        name: Ident,
        overrid: bool,
    },
    Custom {
        name: Ident,
        ty: Type,
        optional: bool,
    },
}

impl Parse for BuilderParam {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let name: Ident = input.parse()?;
        if input.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            let fork = input.fork();
            let start: Ident = fork.parse()?;
            let (ty, optional) = if start == "Option" {
                let _: Ident = input.parse()?;
                let _: Token![<] = input.parse()?;
                let inner_ty: Type = input.parse()?;
                let _: Token![>] = input.parse()?;
                (inner_ty, true)
            } else {
                (input.parse()?, false)
            };
            Ok(Self::Custom { name, ty, optional })
        } else {
            let overrid = input.peek(Token![?]);
            if overrid {
                let _: Token![?] = input.parse()?;
            }
            Ok(Self::Field { name, overrid })
        }
    }
}

struct PartialBuilderInfo {
    vis: Visibility,
    name: Option<Ident>,
    params: Vec<BuilderParam>,
    custom_return_type: Option<Type>,
    return_semantics: ReturnSemantics,
}

struct BuilderInfo {
    vis: Visibility,
    name: Ident,
    params: Vec<BuilderParam>,
    custom_return_type: Option<Type>,
    return_semantics: ReturnSemantics,
}

impl PartialBuilderInfo {
    fn complete(self, struct_name: &Ident) -> BuilderInfo {
        BuilderInfo {
            vis: self.vis,
            name: self.name.unwrap_or(format_ident!("{}Builder", struct_name)),
            params: self.params,
            custom_return_type: self.custom_return_type,
            return_semantics: self.return_semantics,
        }
    }
}

impl Parse for PartialBuilderInfo {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        // An empty input is also a visibility.
        let mut vis: Visibility = input.parse().unwrap();
        let name: Option<Ident> = if input.peek(Ident) {
            Some(input.parse()?)
        } else {
            // If they didn't explicitly give a name default to public visibility.
            vis = parse_quote! { pub };
            None
        };
        let params = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let param_list = content.parse_terminated::<_, Comma>(BuilderParam::parse)?;
            param_list.into_iter().collect()
        } else {
            Vec::new()
        };
        let (custom_return_type, return_semantics) = if input.peek(Token![-]) {
            let _: Token![-] = input.parse()?;
            let _: Token![>] = input.parse()?;
            let fork = input.fork();
            let mut ty: Type = input.parse()?;
            let type_name: Ident = fork.parse()?;
            let semantics = if type_name == "Self" {
                ReturnSemantics::Selff
            } else if type_name == "Result" {
                let _: Token![<] = fork.parse()?;
                let _: Token![Self] = fork.parse()?;
                let _: Token![,] = fork.parse()?;
                let other_type: Type = fork.parse()?;
                let _: Token![>] = fork.parse()?;
                ty = other_type;
                ReturnSemantics::Result
            } else {
                return Err(Error::new_spanned(
                    ty,
                    "This macro can only create constructors that return Self or Result<Self, _>.",
                ));
            };
            (Some(ty), semantics)
        } else {
            (None, ReturnSemantics::Selff)
        };
        Ok(Self {
            vis,
            name,
            params,
            custom_return_type,
            return_semantics,
        })
    }
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
    custom_return_type: Option<Type>,
    return_semantics: ReturnSemantics,
}

impl Parse for ConstructorInfo {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        // An empty input is also a visibility.
        let mut vis: Visibility = input.parse().unwrap();
        let name: Ident = if input.peek(Ident) {
            input.parse()?
        } else {
            // If they didn't explicitly give a name default to public visibility.
            vis = parse_quote! { pub };
            parse_quote! { new }
        };
        let params = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let param_list = content.parse_terminated::<_, Comma>(ConstructorParam::parse)?;
            param_list.into_iter().collect()
        } else {
            vec![ConstructorParam::Ellipses]
        };
        let (custom_return_type, return_semantics) = if input.peek(Token![-]) {
            let _: Token![-] = input.parse()?;
            let _: Token![>] = input.parse()?;
            let fork = input.fork();
            let mut ty: Type = input.parse()?;
            let type_name: Ident = fork.parse()?;
            let semantics = if type_name == "Self" {
                ReturnSemantics::Selff
            } else if type_name == "Result" {
                let _: Token![<] = fork.parse()?;
                let _: Token![Self] = fork.parse()?;
                let _: Token![,] = fork.parse()?;
                let other_type: Type = fork.parse()?;
                let _: Token![>] = fork.parse()?;
                // Make sure we are using the right Result type.
                ty = parse_quote! { ::core::result::Result<Self, #other_type> };
                ReturnSemantics::Result
            } else {
                return Err(Error::new_spanned(
                    ty,
                    "This macro can only create constructors that return Self or Result<Self, _>.",
                ));
            };
            (Some(ty), semantics)
        } else {
            (None, ReturnSemantics::Selff)
        };
        Ok(Self {
            vis,
            name,
            params,
            custom_return_type,
            return_semantics,
        })
    }
}

#[derive(Clone)]
enum BuilderField {
    Required {
        name: Ident,
        ty: Type,
        status_param: Ident,
    },
    Optional {
        name: Ident,
        ty: Type,
    },
    Override {
        name: Ident,
        ty: Type,
    },
}

impl BuilderField {
    fn borrow_name(&self) -> &Ident {
        match self {
            Self::Required { name, .. }
            | Self::Optional { name, .. }
            | Self::Override { name, .. } => name,
        }
    }
}

fn make_builder_fields(
    builder_name: &str,
    params: Vec<BuilderParam>,
    fields: &[FieldInfo],
) -> Result<(Vec<Ident>, Vec<BuilderField>), Error> {
    let mut status_params = Vec::new();
    let mut builder_fields = Vec::new();
    // Stores fields that must be in the parameters of the builder but the user has not
    // yet explicitly specified any extra settings for them.
    let mut remaining_fields: Vec<_> = fields
        .iter()
        .cloned()
        .filter(|e| !e.custom_init.contains_key(builder_name) && e.default_init.is_none())
        .collect();
    for param in params {
        match param {
            BuilderParam::Field { name, overrid } => {
                let mut found_field: Option<FieldInfo> = None;
                for (index, field) in remaining_fields.iter().enumerate() {
                    if field.ident == name {
                        found_field = Some(remaining_fields.remove(index));
                        break;
                    }
                }
                if found_field.is_none() {
                    for field in fields {
                        if field.ident == name {
                            found_field = Some(field.clone());
                            break;
                        }
                    }
                }
                if let Some(field) = found_field {
                    if overrid {
                        builder_fields.push(BuilderField::Override {
                            name,
                            ty: field.ty.clone(),
                        })
                    } else {
                        let status_param =
                            format_ident!("{}Status__", field.ident.to_string().to_pascal_case());
                        status_params.push(status_param.clone());
                        builder_fields.push(BuilderField::Required {
                            name,
                            ty: field.ty.clone(),
                            status_param,
                        })
                    }
                } else {
                    return Err(Error::new_spanned(
                        name,
                        concat!("Could not find a field with this name",),
                    ));
                }
            }
            BuilderParam::Custom { name, ty, optional } => {
                if optional {
                    builder_fields.push(BuilderField::Optional { name, ty });
                } else {
                    let status_param =
                        format_ident!("{}Status__", name.to_string().to_pascal_case());
                    status_params.push(status_param.clone());
                    builder_fields.push(BuilderField::Required {
                        name,
                        ty,
                        status_param,
                    })
                }
            }
        }
    }
    for field in remaining_fields {
        let status_param = format_ident!("{}Status__", field.ident.to_string().to_pascal_case());
        status_params.push(status_param.clone());
        builder_fields.push(BuilderField::Required {
            name: field.ident,
            ty: field.ty.clone(),
            status_param,
        })
    }
    Ok((status_params, builder_fields))
}

fn make_builder_impl(
    struct_name: &Ident,
    generic_params: &Generics,
    info: BuilderInfo,
    fields: &[FieldInfo],
) -> Result<TokenStream2, Error> {
    let builder_name = info.name;
    let str_name = builder_name.to_string();
    let (status_params, builder_fields) = make_builder_fields(&str_name, info.params, fields)?;
    let all_fields = builder_fields.clone();
    let vis = info.vis;
    let generic_args = make_generic_args(generic_params);
    let mut field_defs = Vec::new();
    let mut initial_values = Vec::new();
    let mut field_mutators = Vec::new();
    let mut constructor_setup = Vec::new();
    let mut override_fields = HashSet::new();
    for field in builder_fields {
        match field {
            BuilderField::Optional { name, ty } => {
                field_defs.push(quote! { #name: ::std::option::Option<#ty> });
                initial_values.push(quote! { #name: ::std::option::Option::None });
                field_mutators.push(quote! {
                    #vis fn #name(mut self, value: #ty) -> Self {
                        self.#name = ::std::option::Option::Some(value);
                        self
                    }
                });
                constructor_setup.push(quote! { let #name = self.#name; });
            }
            BuilderField::Override { name, ty } => {
                field_defs.push(quote! { #name: ::std::option::Option<#ty> });
                initial_values.push(quote! { #name: ::std::option::Option::None });
                field_mutators.push(quote! {
                    #vis fn #name(mut self, value: #ty) -> Self {
                        self.#name = ::std::option::Option::Some(value);
                        self
                    }
                });
                constructor_setup.push(quote! { let #name = self.#name; });
                override_fields.insert(name.to_string());
            }
            BuilderField::Required {
                name,
                ty,
                status_param,
            } => {
                field_defs
                    .push(quote! { #name: ::scones::BuilderFieldContainer<#ty, #status_param> });
                initial_values.push(quote! { #name: ::scones::BuilderFieldContainer::missing() });
                // Replace FieldNameStatus__ with ::scones::Present after using the mutator fn.
                let mut sp_after_mut = status_params
                    .iter()
                    .map(|sp| {
                        if sp == &status_param {
                            quote! { ::scones::Present }
                        } else {
                            quote! { #sp }
                        }
                    })
                    .collect();
                let mut new_generic_args = generic_args.clone();
                new_generic_args.append(&mut sp_after_mut);
                let mut mutator_fields = Vec::new();
                for other_field in &all_fields {
                    let other_name = other_field.borrow_name();
                    // If this is the field we are mutating...
                    if other_name == &name {
                        mutator_fields.push(
                            quote! { #name: ::scones::BuilderFieldContainer::present(value) },
                        );
                    } else {
                        mutator_fields.push(quote! { #other_name: self.#other_name });
                    }
                }
                field_mutators.push(quote! {
                    #vis fn #name(self, value: #ty) -> #builder_name <#(#new_generic_args),*> {
                        #builder_name {
                            #(#mutator_fields),*
                        }
                    }
                });
                constructor_setup.push(quote! { let #name = self.#name.into_value(); });
            }
        }
    }

    let mut initializers = Vec::new();
    for field in fields {
        let ident = &field.ident;
        let init = field
            .custom_init
            .get(&str_name)
            .or(field.default_init.as_ref())
            .cloned()
            .unwrap_or(quote! { #ident });
        if override_fields.contains(&ident.to_string()) {
            initializers.push(quote! {
                #ident: #ident.unwrap_or(#init)
            });
        } else {
            initializers.push(quote! {
                #ident: #init
            });
        }
    }

    let all_missing_args = {
        let mut vec = generic_args.clone();
        vec.append(
            &mut status_params
                .iter()
                .map(|_| quote! { ::scones::Missing })
                .collect(),
        );
        vec
    };
    let all_present_args = {
        let mut vec = generic_args.clone();
        vec.append(
            &mut status_params
                .iter()
                .map(|_| quote! { ::scones::Present })
                .collect(),
        );
        vec
    };
    let all_generic_args = {
        let mut vec = generic_args.clone();
        vec.append(&mut status_params.iter().map(|i| quote! { #i }).collect());
        vec
    };
    let result_type: Type = parse_quote! { #struct_name <#(#generic_args),*> };
    let mut return_type = info.custom_return_type.unwrap_or(result_type.clone());
    let return_semantics = info.return_semantics;
    let constructor_body = match return_semantics {
        ReturnSemantics::Selff => quote! { #struct_name { #(#initializers),* } },
        ReturnSemantics::Result => {
            return_type = parse_quote! { ::core::result::Result<#result_type, #return_type> };
            quote! { ::core::result::Result::Ok(#struct_name { #(#initializers),* }) }
        }
    };
    let generic_where = &generic_params.where_clause;
    let mut all_generic_params = generic_params.clone();
    for status_param in &status_params {
        all_generic_params
            .params
            .push(parse_quote! { #status_param });
    }

    let mut documentation = "".to_owned();
    documentation.push_str(&format!(
        "A builder which creates an instance of `{}`. Use `{}::new()` to start the builder. ",
        struct_name, builder_name,
    ));
    documentation.push_str("Calling `build()` consumes the builder, returning the completed ");
    documentation.push_str("item. Before calling `build()`, you can modify values the builder ");
    documentation.push_str("will use by calling any of the other functions. For this builder, ");
    documentation.push_str("you must call all of the following functions at least once before ");
    documentation.push_str("calling `build()`, or you will receive a compilation error:\n");
    let mut example = format!("");
    for field in &all_fields {
        if let BuilderField::Required { name, ty, .. } = field {
            documentation.push_str(&format!("- `{}(value: {})`\n", name, quote! { #ty }));
            example.push_str(&format!("\n    .{}(value)", name));
        }
    }
    documentation.push_str("\nHere is a minimal example:\n```\n");
    documentation.push_str(&format!(
        "let instance = {}::new(){}.build();\n```",
        builder_name, example,
    ));

    Ok(quote! {
        #[doc=#documentation]
        #vis struct #builder_name #all_generic_params #generic_where {
            #(#field_defs),*
        }
        impl #generic_params #builder_name <#(#all_missing_args),*> #generic_where {
            #vis fn new() -> Self {
                Self {
                    #(#initial_values),*
                }
            }
        }
        impl #all_generic_params #builder_name <#(#all_generic_args),*> #generic_where {
            #(#field_mutators)*
        }
        impl #generic_params #builder_name <#(#all_present_args),*> #generic_where {
            #vis fn build(self) -> #return_type {
                #(#constructor_setup)*
                #constructor_body
            }
        }
    })
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
    let return_type = info.custom_return_type.unwrap_or(parse_quote! { Self });
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
    let body = match info.return_semantics {
        ReturnSemantics::Selff => quote! {
            Self {
                #(#initializers),*
            }
        },
        ReturnSemantics::Result => quote! {
            ::core::result::Result::Ok(Self {
                #(#initializers),*
            })
        },
    };
    Ok(quote! {
        #vis fn #name (#params) -> #return_type {
            #body
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

fn make_generic_args(params: &Generics) -> Vec<TokenStream2> {
    let mut args = Vec::new();
    for param in params.params.iter() {
        match param {
            GenericParam::Type(tp) => {
                let ident = &tp.ident;
                args.push(quote! { #ident });
            }
            GenericParam::Lifetime(lt) => {
                args.push(quote! { #lt });
            }
            GenericParam::Const(cp) => {
                let ident = &cp.ident;
                args.push(quote! { #ident });
            }
        }
    }
    args
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

// This can be invoked multiple times and it will produce a single #[generate_items__]
// invocation.
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

// This can be invoked multiple times and it will produce a single #[generate_items__]
// invocation.
#[proc_macro_attribute]
pub fn make_builder(input_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_attr2: TokenStream2 = input_attr.clone().into();
    // Check that the input is valid.
    let _: PartialBuilderInfo = syn::parse_macro_input!(input_attr);
    let macro_arg = quote! { builder { #input_attr2 } };
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
    builders: Vec<PartialBuilderInfo>,
    constructors: Vec<ConstructorInfo>,
}

impl Parse for GenerateItemsArgs {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut result = Self {
            builders: Vec::new(),
            constructors: Vec::new(),
        };
        while !input.is_empty() {
            let kind: Ident = input.parse()?;
            let content;
            braced!(content in input);
            if kind == "constructor" {
                result.constructors.push(content.parse()?);
            } else if kind == "builder" {
                result.builders.push(content.parse()?);
            } else {
                unreachable!("Bad syntax generation");
            }
        }
        Ok(result)
    }
}

/// This is the actual macro that generates constructors. Use #{make_constructor} to invoke it.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn generate_items__(attr: TokenStream, item: TokenStream) -> TokenStream {
    let GenerateItemsArgs {
        builders,
        constructors,
    } = syn::parse_macro_input!(attr);
    let mut item_names: HashSet<String> = HashSet::new();
    for c in &constructors {
        item_names.insert(c.name.to_string());
    }
    let mut struct_def: ItemStruct = syn::parse_macro_input!(item);
    let generic_params = &struct_def.generics;
    let struct_name = &struct_def.ident;
    let builders: Vec<_> = builders
        .into_iter()
        .map(|b| b.complete(struct_name))
        .collect();
    for b in &builders {
        item_names.insert(b.name.to_string());
    }

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

    let mut builder_code = Vec::new();
    for builder in builders {
        match make_builder_impl(&struct_name, &generic_params, builder, &field_infos[..]) {
            Ok(def) => builder_code.push(def),
            Err(err) => return err.to_compile_error().into(),
        }
    }
    let mut constructor_defs = Vec::new();
    for cons in constructors {
        match make_constructor_impl(cons, &field_infos[..]) {
            Ok(def) => constructor_defs.push(def),
            Err(err) => return err.to_compile_error().into(),
        }
    }

    let generic_param_list = &generic_params.params;
    let generic_where = &generic_params.where_clause;
    let generic_args = make_generic_args(&generic_params);

    (quote! {
        #struct_def
        #(#builder_code)*
        impl <#generic_param_list> #struct_name <#(#generic_args),*> #generic_where {
            #(#constructor_defs)*
        }
    })
    .into()
}
