use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_error::abort;
use quote::{quote, spanned::Spanned};
use syn::{Fields, ItemEnum, Variant};

use crate::util::{field_transform, untyped_fields};

fn variant_ser_struct(variant: &Variant) -> TokenStream2 {
  let fields = if let Fields::Named(fields) = &variant.fields {
    fields
  } else {
    abort!(
      variant.fields.__span(),
      "Variants of emit messages must be named structs."
    );
  };

  // Transform the named fields into reference-only fields.
  let ref_struct_fields = field_transform(fields, |field| syn::Field {
    ty: syn::Type::Reference(syn::TypeReference {
      and_token: syn::token::And {
        spans: [Span::call_site()],
      },
      lifetime: Some(syn::Lifetime::new("'a", Span::call_site())),
      mutability: None,
      elem: Box::new(field.ty.clone()),
    }),
    ..field.clone()
  });

  quote! {
    #[derive(::serde::Serialize)]
    #[serde(deny_unknown_fields)]
    struct AsyncSocketInternalRespondEvent<'a> {
      #ref_struct_fields
    }
  }
}

fn gen_serialize(item: &ItemEnum) -> TokenStream2 {
  let enum_name = &item.ident;

  if item.variants.is_empty() {
    return quote! {
      match *self {}
    };
  }

  let branches = item
    .variants
    .iter()
    .fold(TokenStream2::new(), |tokens, branch| {
      let name = &branch.ident;
      let fields = if let Fields::Named(fields) = &branch.fields {
        untyped_fields(fields)
      } else {
        abort!(
          branch.fields.__span(),
          "Variants of emit messages must be named structs."
        );
      };

      let ser_struct = variant_ser_struct(branch);
      quote! {
        #tokens
        #enum_name :: #name { #fields } => {
          #ser_struct;
          AsyncSocketInternalRespondEvent { #fields }.serialize(serializer)
        }
      }
    });

  quote! {
    match self {
      #branches
    }
  }
}

pub fn gen_responders(item: ItemEnum) -> TokenStream2 {
  let enum_name = &item.ident;
  let serialize = gen_serialize(&item);

  quote! {
    impl ::serde::ser::Serialize for #enum_name {
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
        S: ::serde::Serializer,
      {
        #serialize
      }
    }

    impl ::async_sockets::SerMessage for #enum_name {}
  }
}
