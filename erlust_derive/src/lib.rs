#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{
    fold::{fold_pat, Fold},
    synom::Synom,
    token::Underscore,
    Block, Expr, Pat, PatWild, Type,
};

#[derive(Clone)]
enum BlockOrExpr {
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

#[derive(Clone)]
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

// Transforms all potential moves into _ ignorers
struct PatIgnorer();

impl Fold for PatIgnorer {
    fn fold_pat(&mut self, p: Pat) -> Pat {
        use self::Pat::*;
        match p {
            Ident(p) => match p.subpat {
                Some((_at, subpat)) => self.fold_pat(*subpat),
                None => Wild(PatWild {
                    underscore_token: Underscore::new(Span::call_site()),
                }),
            },
            p => p,
        }
    }
}

fn gen_arm_ident(i: usize) -> Ident {
    // TODO: (B) Make this Span::def_site h:proc-macro-extras
    Ident::new(&format!("Arm{}", i), Span::call_site())
}

fn gen_inner_match(i: usize, ty: Type, pat: Pat, guard: TokenStream) -> TokenStream {
    let arm_name = gen_arm_ident(i);
    quote! {
        msg = match msg.downcast::<#ty>() {
            Ok(msg) => {
                let matches = match &mut *msg {
                    &mut #pat #guard => true,
                    _ => false,
                };
                if matches {
                    return ::erlust::ReceiveResult::Use(MatchedArm::#arm_name(msg));
                }
                msg as Box<Any>
            },
            Err(msg) => msg,
        };
    }
}

fn gen_outer_match_arm(i: usize, pat: Pat, body: BlockOrExpr) -> TokenStream {
    let arm_name = gen_arm_ident(i);
    quote! {
        #arm_name(msg) => match *msg {
            #pat => #body,
            _ => unreachable!() // TODO: (B) consider unreachable_unchecked
        },
    }
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
pub fn receive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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

    // Generate the MatchedArm enum
    let names_and_types = parsed.arms.iter().enumerate().map(|(i, arm)| {
        let name = Ident::new(&format!("Arm{}", i), Span::call_site());
        let ty = arm.ty.clone();
        quote!(#name(#ty))
    });
    let arms_def = quote!(
        enum MatchedArm {
            #(#names_and_types ,)*
        }
    );

    // Generate the inner matches
    let inner_matches = parsed.arms.iter().cloned().enumerate().map(|(i, arm)| {
        if let Some(guard) = arm.guard {
            gen_inner_match(i, arm.ty, arm.pat, quote!(if #guard))
        } else {
            let ignoring_pat = fold_pat(&mut PatIgnorer(), arm.pat);
            gen_inner_match(i, arm.ty, ignoring_pat, quote!())
        }
    });

    // Generate the outer match's arms
    let outer_match_arms = parsed
        .arms
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, arm)| gen_outer_match_arm(i, arm.pat, arm.body));

    let res = quote! {
        {
            #arms_def

            match ::erlust::receive(|mut msg| {
                #(#inner_matches)*
                ::erlust::ReceiveResult::Skip(msg)
            }) {
                #(#outer_match_arms)*
            }
        }
    };
    res.into()
}

// TODO: (A) receive_box
