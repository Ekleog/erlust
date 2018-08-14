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

// Being given:
//
//  receive! {
//      (usize, String): (1, ref x) if foo(x) => bar(x),
//      (usize, String): (2, x) => foobar(x),
//      usize: x if baz(x) => quux(x),
//  }
//
// With types:
//  * `foo`:    `Fn(&String) -> bool`
//  * `bar`:    `Fn(&String) -> T`
//  * `foobar`: `Fn(String) -> T`
//  * `baz`:    `Fn(usize) -> bool`
//  * `quux`:   `Fn(usize) -> T`
//
// Expands to:
//
//  enum MatchedArm {
//      Arm1(Box<(usize, String)>),
//      Arm2(Box<(usize, String)>),
//      Arm3(Box<usize>),
//  }
//  match receive(|mut msg: LocalMessage| {
//      msg = match msg.downcast::<(usize, String)>() {
//          Ok(res) => {
//  [has match guard, thus cannot move, thus mutable borrow]
//              let matches = match &mut *res {
//                  &mut (1, ref x) if foo(x) => true,
//                  _ => false,
//              };
//              if matches {
//                  return Use(Arm1(res));
//              }
//              res as Box<Any>
//          },
//          Err(res) => res,
//      };
//      msg = match msg.downcast::<(usize, String)>() {
//          Ok(res) => {
//  [has no match guard, thus can move, but we can just ignore it here]
//              let matches = match &*res {
//                  &(2, _) => true,
//                  _ => false,
//              };
//              if matches {
//                  return Use(Arm2(res));
//              }
//              res as Box<Any>
//          },
//          Err(b) => b,
//      };
//      msg = match msg.downcast::<usize>() {
//          Ok(res) => {
//  [has a match guard, thus cannot move, thus mutable borrow]
//              let matches = match &mut *res {
//                  &mut x if baz(x) => true,
//                  _ => false,
//              };
//              if matches {
//                  return Use(Arm3(res));
//              }
//              res as Box<Any>
//          },
//      };
//      Skip(msg)
//  }) {
//      Arm1(msg) => match *msg {
//          (1, ref x) => bar(x),
//          _ => unreachable!(),
//      },
//      Arm2(msg) => match *msg {
//          (2, x) => foobar(x),
//          _ => unreachable!(),
//      },
//      Arm3(msg) => match *msg {
//          x => quux(x),
//          _ => unreachable!(),
//      },
//  }

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
    let res = quote!{};
    res.into()
}
