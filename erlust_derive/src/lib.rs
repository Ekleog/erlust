#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use syn::{Type, Pat, Expr, Block};

struct Receive {
    ty: Type,
    pat: Pat,
    guard: Option<Expr>,
    body: Option<Block>,
}

#[proc_macro]
pub fn receive(input: TokenStream) -> TokenStream {
    let parsed = syn::parse::<Receive>(input);
    let res = quote! {
        crate::foo(3)
    };
    res.into()
}
