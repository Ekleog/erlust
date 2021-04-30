use proc_macro2::TokenStream;
use syn::{spanned::Spanned, DeriveInput, Meta};

pub fn derive_message(input: TokenStream) -> TokenStream {
    let s = syn::parse2::<DeriveInput>(input).unwrap();

    let mut tag = None;
    for attr in s.attrs {
        if let Ok(Meta::NameValue(m)) = attr.parse_meta() {
            if m.path == syn::parse_str("erlust_tag").unwrap() {
                if tag.is_some() {
                    attr.span()
                        .unstable()
                        .error("Used the `erlust_tag` attribute multiple times")
                        .emit();
                    return TokenStream::new();
                }
                tag = Some(m.lit);
            }
        }
    }

    if let Some(tag) = tag {
        let name = s.ident;
        let res = quote! {
            impl ::erlust::Message for #name {
                fn tag() -> &'static str {
                    #tag
                }
            }
        };
        res.into()
    } else {
        s.ident
            .span()
            .unstable()
            .error("Missing `erlust_tag` attribute")
            .emit();
        return TokenStream::new();
    }
}
