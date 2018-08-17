use proc_macro2::TokenStream;
use syn::{spanned::Spanned, DeriveInput, Meta};

pub fn derive_message(s: DeriveInput) -> TokenStream {
    let mut tag = None;
    for attr in s.attrs {
        if let Some(Meta::NameValue(m)) = attr.interpret_meta() {
            if m.ident == "erlust_tag" {
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
