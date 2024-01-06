use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::abort;
use quote::spanned::Spanned;
use quote::{quote, ToTokens};
use syn::{Field, FieldsNamed};

pub fn field_transform<F, U>(fields_named: &FieldsNamed, visitor: F) -> TokenStream2
where
  F: Fn(&Field) -> U,
  U: ToTokens,
{
  fields_named.named.iter().map(visitor).fold(
    TokenStream2::new(),
    |tokens, ident| quote! { #tokens #ident, },
  )
}

/// Transforms FieldsNamed (looks like `{ field1: Type1, field2: Type2, ... }`)
/// into `field1, field2, ...,`.
pub fn untyped_fields(fields_named: &FieldsNamed) -> TokenStream2 {
  field_transform(fields_named, |field| {
    if let Some(ident) = &field.ident {
      quote! { #ident }
    } else {
      abort!(field.__span(), "Expected identifier on all parameters.");
    }
  })
}

/// Transforms FieldsNamed (looks like `{ field1: Type1, field2: Type2, ... }`)
/// into `Type1, Type2, ...,`.
pub fn field_types(fields_named: &FieldsNamed) -> TokenStream2 {
  field_transform(fields_named, |field| {
    let ty = &field.ty;
    quote! { #ty }
  })
}
