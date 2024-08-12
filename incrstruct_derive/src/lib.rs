//! The macros used to generate self-referencing structs.

use std::collections::HashSet;

extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error};
use syn::spanned::Spanned;

/// Derives initialization functions for a struct. See the
/// [crate documentation](../incrstruct).
#[proc_macro_derive(IncrStruct, attributes(borrows, header))]
pub fn derive_incr_struct(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);

    match incr_struct(&input) {
        Ok(output) => output,
        Err(err) => err.into_compile_error().into(),
    }
}

fn incr_struct(input: &DeriveInput) -> Result<TokenStream, Error> {
    let data_struct = match &input.data {
        syn::Data::Struct(data) => Ok(data),
        syn::Data::Enum(data) => Err(data.enum_token.span()),
        syn::Data::Union(data) => Err(data.union_token.span()),
    }
    .map_err(|span| Error::new(span, "IncrStruct can only be used on structs"))?;

    let mut fields = get_named_fields(&input);

    let header = if let Some(header) = fields.pop() {
        if !has_attribute(&header.attrs, "header") {
            return Err(Error::new_spanned(
                header,
                "missing #[header] attribute on last field",
            ));
        }

        header
    } else {
        return Err(Error::new_spanned(
            &data_struct.fields,
            "missing #[header] field",
        ));
    };
    let header_name = header.ident.as_ref().unwrap();

    // We are mostly concerned with initialization, which means heads
    // before tails. We simply reverse the list (now that header is
    // removed) and go with that. Rust doesn't have the concept of
    // initialization order the way C++ does, but this crate is
    // introducing it.
    fields.reverse();

    let heads = find_phase(fields.as_slice(), false);
    let tails = find_phase(fields.as_slice(), true);

    let head_params = make_field_params(heads.as_slice(), None);
    let head_args = make_field_args(heads.as_slice(), None);
    let tail_names = make_field_args(tails.as_slice(), None);

    // Drop order is the reverse of the reverse.
    let mut drop_head_names = head_args.clone();
    drop_head_names.reverse();
    let mut drop_tail_names = tail_names.clone();
    drop_tail_names.reverse();

    let (generics_decls, generics_args, generics_where) = input.generics.split_for_impl();
    let first_lifetime = input
        .generics
        .lifetimes()
        .nth(0)
        .map(|param| &param.lifetime);

    let (init_field_decls, init_field_args) = make_init_field_decls_and_args(
        fields.as_slice(),
        first_lifetime,
        Some(&syn::Ident::new("r", proc_macro2::Span::call_site())),
    )?;
    let init_field_names = make_init_field_names(tails.as_slice());

    let struct_name = &input.ident;
    let init_trait_name = syn::Ident::new(
        &(struct_name.to_string() + "Init"),
        proc_macro2::Span::call_site(),
    );

    Ok(quote! {
        impl #generics_decls #struct_name #generics_args #generics_where {
            pub fn new_box(#(#head_params),*) -> std::boxed::Box<Self> {
                // SAFETY: the callee is aware the struct is partially initialized.
                incrstruct::new_box(unsafe { Self::new_uninit(#(#head_args),*) })
            }

            pub fn new_rc(#(#head_params),*) -> std::rc::Rc<Self> {
                // SAFETY: the callee is aware the struct is partially initialized.
                incrstruct::new_rc(unsafe { Self::new_uninit(#(#head_args),*) })
            }

            /// See `iterstruct::new_uninit`.
            pub unsafe fn new_uninit(#(#head_params),*) -> core::mem::MaybeUninit<Self> {
                // SAFETY: we only write each field once, so this
                // overwrites uninitialized values.
                incrstruct::new_uninit::<Self, _>(|out| unsafe {
                    #(
                        core::ptr::write(&mut out.#head_args, #head_args);
                    )*
                })
            }

            /// Drops a value previously created with `new_uninit`.
            fn drop_uninit_in_place(this: core::mem::MaybeUninit<Self>) {
                incrstruct::drop_uninit_in_place(this, |this| {
                    // SAFETY: we only drop head fields, and only once.
                    unsafe {
                        #(
                            core::ptr::drop_in_place(&mut this.#drop_head_names);
                        )*
                    };
                });
            }
        }

        trait #init_trait_name #generics_decls #generics_where {
            #(
                #init_field_decls
            )*
        }

        impl #generics_decls incrstruct::IncrStructInit for #struct_name #generics_args #generics_where {
            unsafe fn init(this: *mut Self) {
                let r = &mut *this;

                // SAFETY: since we only support referencing earlier
                // fields, in a DAG, this always writes to
                // uninitialized space. The generated trait guarantees
                // that init_field_X is not unsafe.

                #(
                    core::ptr::write(&mut r.#tail_names as *mut _, <Self as #init_trait_name #generics_args>::#init_field_names(#( #init_field_args ),*));
                )*
            }

            /// Drops all tail fields, returning a partially initialized struct.
            ///
            /// # Safety
            ///
            /// We only drop tail fields, and only once.
            unsafe fn drop_tail_in_place(this: &mut Self) {
                #( core::ptr::drop_in_place(&mut this.#drop_tail_names); )*
            }

            fn header<'isheader>(this: &'isheader mut Self) -> &'isheader mut incrstruct::Header {
                &mut this.#header_name
            }
        }
    }
    .into())
}

/// Returns a list of function parameters, like how a function is declared.
fn make_field_params(
    fields: &[&syn::Field],
    ref_lifetime: Option<&syn::Lifetime>,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.ident;
            let ty = &field.ty;
            if let Some(ref ref_lifetime) = ref_lifetime {
                quote! { #name: & #ref_lifetime #ty }.into()
            } else {
                quote! { #name: #ty }.into()
            }
        })
        .collect()
}

/// Returns a list of argument names, like how a function is invoked.
fn make_field_args(
    fields: &[&syn::Field],
    src: Option<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.ident;
            if let Some(src) = src {
                quote! { &#src.#name }.into()
            } else {
                quote! { #name }.into()
            }
        })
        .collect()
}

fn make_init_field_names(fields: &[&syn::Field]) -> Vec<syn::Ident> {
    fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap().to_string();

            syn::Ident::new(
                &("init_field_".to_string() + name.as_str()),
                proc_macro2::Span::call_site(),
            )
        })
        .collect()
}

fn make_init_field_decls_and_args(
    fields: &[&syn::Field],
    ref_lifetime: Option<&syn::Lifetime>,
    src: Option<&syn::Ident>,
) -> Result<
    (
        Vec<proc_macro2::TokenStream>,
        Vec<Vec<proc_macro2::TokenStream>>,
    ),
    Error,
> {
    let mut decls = Vec::new();
    let mut args = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        if !has_attribute(&field.attrs, "borrows") {
            continue;
        }

        let ty = &field.ty;
        let name = field.ident.as_ref().unwrap().to_string();
        let fn_name = syn::Ident::new(
            &("init_field_".to_string() + name.as_str()),
            proc_macro2::Span::call_site(),
        );
        let borrows = get_borrows(field)?;
        let param_fields = find_borrows_fields(&fields[..i], borrows).map_err(|missing| {
            let mut out: Option<Error> = None;

            for dep in missing.into_iter() {
                let err = Error::new_spanned(&dep, "borrowed field not found later in the struct");
                if let Some(ref mut out) = out {
                    out.combine(err);
                } else {
                    out = Some(err);
                }
            }

            out.unwrap()
        })?;
        let params = make_field_params(param_fields.as_slice(), ref_lifetime);

        decls.push(quote! { fn #fn_name(#( #params ),*) -> #ty; }.into());
        args.push(make_field_args(param_fields.as_slice(), src));
    }

    Ok((decls, args))
}

fn find_borrows_fields<'b>(
    fields: &'b [&syn::Field],
    mut borrows: HashSet<syn::Ident>,
) -> Result<Vec<&'b syn::Field>, HashSet<syn::Ident>> {
    let out = fields
        .iter()
        .map(|field| *field)
        .filter(|field| field.ident.is_some() && borrows.remove(field.ident.as_ref().unwrap()))
        .collect();

    if borrows.is_empty() {
        Ok(out)
    } else {
        Err(borrows)
    }
}

fn get_borrows(field: &syn::Field) -> Result<HashSet<syn::Ident>, Error> {
    let attr = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("borrows"));

    if let Some(attr) = attr {
        let args = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
        )?;

        Ok(HashSet::from_iter(args.into_iter()))
    } else {
        Ok(HashSet::new())
    }
}

/// Returns the fields of the struct that can be initialized directly,
/// in phase one. These are called heads in Ouroboros.
fn find_phase<'b>(fields: &'b [&syn::Field], borrows: bool) -> Vec<&'b syn::Field> {
    fields
        .iter()
        .map(|field| *field)
        .filter(|field| has_attribute(&field.attrs, "borrows") == borrows)
        .collect()
}

fn get_named_fields(input: &DeriveInput) -> Vec<&syn::Field> {
    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.named.iter().collect(),
            syn::Fields::Unnamed(_) => Vec::new(),
            syn::Fields::Unit => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn has_attribute(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}
