#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use syn::{synom::Synom, Block, Expr, Pat, Type};

enum BlockOrExpr {
    Block(Block),
    Expr(Expr),
}

struct ReceiveArm {
    ty:    Type,
    pat:   Pat,
    guard: Option<Expr>,
    body:  BlockOrExpr,
}

impl Synom for ReceiveArm {
    named!(parse -> Self, do_parse!(
        ty: syn!(Type) >>
        punct!(:) >>
        pat: syn!(Pat) >>
        guard: option!(
            do_parse!(
                keyword!(if) >>
                g: syn!(Expr) >>
                (g)
            )
        ) >>
        punct!(=>) >>
        body: alt!(
            do_parse!(
                b: syn!(Block) >>
                option!(punct!(,)) >>
                (BlockOrExpr::Block(b))
            ) |
            do_parse!(
                e: syn!(Expr) >>
                punct!(,) >>
                (BlockOrExpr::Expr(e))
            )
        ) >>
        (ReceiveArm { ty, pat, guard, body })
    ));
}

struct Receive {
    arms: Vec<ReceiveArm>,
}

impl Synom for Receive {
    named!(parse -> Self, do_parse!(
        arms: many0!(syn!(ReceiveArm)) >>
        (Receive { arms })
    ));
}

#[proc_macro]
pub fn receive(input: TokenStream) -> TokenStream {
    let parsed = syn::parse::<Receive>(input).expect(
        "Failed to parse receive! block.

Reminder: syntax is as follows:
```
receive! {
    (usize, String): (1, s) => foo(s),
    usize: x if bar(x) => { baz(x) }
}
```
",
    );
    let res = quote! {
        crate::foo(3)
    };
    res.into()
}
