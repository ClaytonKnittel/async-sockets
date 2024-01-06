use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::abort;
use quote::quote;
use quote::spanned::Spanned;
use syn::{Fields, ItemEnum};

use crate::util::{field_types, untyped_fields};

pub fn gen_listeners(item: ItemEnum) -> TokenStream2 {
  let enum_name = item.ident;

  let gen_variants = item
    .variants
    .iter()
    .fold(TokenStream2::new(), |tokens, branch| {
      let name = &branch.ident;
      let types = if let Fields::Named(fields) = &branch.fields {
        field_types(fields)
      } else {
        abort!(
          branch.fields.__span(),
          "Variants of emit messages must be named structs."
        );
      };

      quote! {
        #tokens
        #name (( #types )),
      }
    });

  let translators = item
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

      quote! {
        #tokens
        AsyncSocketInternalListenerEvent:: #name (( #fields )) => Ok( #enum_name :: #name { #fields } ),
      }
    });

  quote! {
    impl<'de> ::serde::de::Deserialize<'de> for #enum_name {
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: ::serde::Deserializer<'de>,
      {
        #[derive(::serde::Deserialize)]
        #[serde(rename_all = "snake_case", tag = "event", content = "args", deny_unknown_fields)]
        enum AsyncSocketInternalListenerEvent {
          #gen_variants
        }

        let proxy = AsyncSocketInternalListenerEvent::deserialize(deserializer)?;

        match proxy {
          #translators
        }
      }
    }

    impl ::async_sockets::DeMessage for #enum_name {}
  }
}
