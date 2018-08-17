use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Block, Expr};

#[derive(Clone)]
pub enum BlockOrExpr {
    Block(Block),
    Expr(Expr),
}

impl ToTokens for BlockOrExpr {
    fn to_tokens(&self, t: &mut TokenStream) {
        use self::BlockOrExpr::*;
        match self {
            Block(b) => b.to_tokens(t),
            Expr(e) => e.to_tokens(t),
        }
    }
}
