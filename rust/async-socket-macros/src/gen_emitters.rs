use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::abort;
use quote::{quote, spanned::Spanned};
use syn::{Fields, ItemEnum, Variant};

use crate::util::{field_transform, untyped_fields};

fn variant_ser_struct(variant: &Variant) -> TokenStream2 {
  let name = &variant.ident;

  let fields = if let Fields::Named(fields) = &variant.fields {
    fields
  } else {
    abort!(
      variant.fields.__span(),
      "Variants of emit messages must be named structs."
    );
  };

  // Transform the named fields into a tuple-type, preseving the order of the
  // fields.
  let tuple_args = field_transform(fields, |field| {
    let ty = &field.ty;
    quote! { &'a #ty }
  });
  let tuple_args = quote! {
    (#tuple_args)
  };

  quote! {
    #[derive(::serde::Serialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum AsyncSocketInternalEmitEventType {
      #name,
    }

    #[derive(::serde::Serialize)]
    #[serde(deny_unknown_fields)]
    struct AsyncSocketInternalEmitEvent<'a> {
      event: AsyncSocketInternalEmitEventType,
      args: #tuple_args,
      #[serde(skip)]
      _p: ::core::marker::PhantomData::<&'a u8>,
    }

    impl<'a> AsyncSocketInternalEmitEvent<'a> {
      fn new(args: #tuple_args) -> Self {
        Self {
          event: AsyncSocketInternalEmitEventType::#name,
          args,
          _p: ::core::marker::PhantomData
        }
      }
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
          AsyncSocketInternalEmitEvent::new(( #fields )).serialize(serializer)
        }
      }
    });

  quote! {
    match self {
      #branches
    }
  }
}

pub fn gen_emitters(item: ItemEnum) -> TokenStream2 {
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
