use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    fold::fold_pat,
    parse::{Parse, ParseStream},
    Expr, Pat, Type,
};

use crate::{block_or_expr::BlockOrExpr, pat_ignorer::PatIgnorer};

#[derive(Clone)]
struct ReceiveArm {
    ty:    Type,
    pat:   Pat,
    guard: Option<Expr>,
    body:  BlockOrExpr,
}

impl Parse for ReceiveArm {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        // type: pattern
        let ty = input.parse()?;
        let _: Token![:] = input.parse()?;
        let pat = input.parse()?;

        // [if foo]
        let guard = if input.peek(Token![if]) {
            let _: Token![if] = input.parse()?;
            Some(input.parse()?)
        } else {
            None
        };

        // => body
        let _: Token![=>] = input.parse()?;
        let body = if input.peek(syn::token::Brace) {
            let res = input.parse()?;
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
            BlockOrExpr::Block(res)
        } else {
            let res = input.parse()?;
            let _: Token![,] = input.parse()?;
            BlockOrExpr::Expr(res)
        };
        Ok(ReceiveArm {
            ty,
            pat,
            guard,
            body,
        })
    }
}

struct Receive {
    arms: Vec<ReceiveArm>,
}

impl Parse for Receive {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut arms = Vec::new();
        while !input.is_empty() {
            arms.push(input.parse()?);
        }
        Ok(Receive { arms })
    }
}

fn gen_arm_ident(i: usize) -> Ident {
    // TODO: (B) Make this Span::def_site h:proc-macro-extras
    Ident::new(&format!("Arm{}", i), Span::call_site())
}

fn gen_local_match(i: usize, ty: Type, pat: Pat, guard: TokenStream) -> TokenStream {
    let arm_name = gen_arm_ident(i);
    quote! {
        if msg.as_any().is::<#ty>() {
            msg = match msg.into_any().downcast::<#ty>() {
                Ok(msg) => {
                    let matches = match (&from, &*msg) {
                        #pat #guard => true,
                        _ => false,
                    };
                    if matches {
                        return ::erlust::ReceiveResult::Use(MatchedArm::#arm_name((from, msg)));
                    }
                    msg as ::erlust::LocalMessage
                },
                Err(msg) => unreachable!(), // TODO: (B) unreachable_unchecked()?
            };
        }
    }
}

fn gen_remote_match(i: usize, ty: Type, pat: Pat, guard: TokenStream) -> TokenStream {
    let arm_name = gen_arm_ident(i);
    quote! {
        if m.tag == <#ty as ::erlust::Message>::tag() {
            let mut deserializer = from.__theater_assert_remote().deserializer(&m.msg);
            match ::erased_serde::deserialize::<Box<#ty>>(&mut deserializer) {
                Ok(msg) => {
                    let matches = match (&from, &*msg) {
                        #pat #guard => true,
                        _ => false,
                    };
                    if matches {
                        return ::erlust::ReceiveResult::Use(MatchedArm::#arm_name((from, msg)));
                    }
                }
                _ => (),
            }
        }
    }
}

fn gen_execute_match_arm(i: usize, pat: Pat, body: BlockOrExpr) -> TokenStream {
    let arm_name = gen_arm_ident(i);
    quote! {
        MatchedArm::#arm_name((from, msg)) => match (from, *msg) {
            #pat => #body,
            _ => unreachable!() // TODO: (B) consider unreachable_unchecked
        },
    }
}

// TODO: (A) handle timeout

// TODO: (A) make tuples and base types implement Message?
// TODO: (B) think of the compatibility-with-old-messages story
// Being given:
//
//  receive! {
//      (usize, String): (_pid, (1, ref x)) if foo(x) => bar(x),
//      (usize, String): (_pid, (2, x)) => foobar(x),
//      usize: (_pid, x) if baz(x) => quux(x),
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
//      Arm1((Pid, Box<(usize, String)>)),
//      Arm2((Pid, Box<(usize, String)>)),
//      Arm3((Pid, Box<usize>)),
//  }
//  match await!(receive(async move |mut msg: ReceivedMessage| {
//      match msg {
//          ReceivedMessage::Local((from, msg)) => {
//              msg = match msg.downcast::<(usize, String)>() {
//                  Ok(res) => {
//          [has match guard, thus cannot move, thus mutable borrow]
//                      let matches = match &mut *res {
//                          &mut (1, ref x) if foo(x) => true,
//                          _ => false,
//                      };
//                      if matches {
//                          return Use(Arm1((from, res)));
//                      }
//                      res as LocalMessage
//                  },
//                  Err(res) => res,
//              };
//              msg = match msg.downcast::<(usize, String)>() {
//                  Ok(res) => {
//          [has no match guard, thus can move, but we can just ignore it here]
//                      let matches = match &*res {
//                          &(2, _) => true,
//                          _ => false,
//                      };
//                      if matches {
//                          return Use(Arm2((from, res)));
//                      }
//                      res as LocalMessage
//                  },
//                  Err(b) => b,
//              };
//              msg = match msg.downcast::<usize>() {
//                  Ok(res) => {
//          [has a match guard, thus cannot move, thus mutable borrow]
//                      let matches = match &mut *res {
//                          &mut x if baz(x) => true,
//                          _ => false,
//                      };
//                      if matches {
//                          return Use(Arm3((from, res)));
//                      }
//                      res as LocalMessage
//                  },
//                  Err(b) => b,
//              };
//              Skip(ReceivedMessage::Local((from, msg)))
//          },
//          ReceivedMessage::Remote((from, m)) => {
//              if m.tag == <(usize, String) as Message>::tag() {
//                  match ::erased_serde::deserialize::<Box<(usize, String)>>(&m.msg) {
//                      Ok(msg) => {
//                          let matches = match &mut *msg {
//                              &mut (1, ref x) if foo(x) => true,
//                              _ => false,
//                          };
//                          if matches {
//                              return Use(Arm1((from, msg)));
//                          }
//                      }
//                      _ => (),
//                  }
//              }
//              if m.tag == <(usize, String) as Message>::tag() {
//                  match ::erased_serde::deserialize::<Box<(usize, String)>>(&m.msg) {
//                      Ok(msg) => {
//                          let matches = match &mut *msg {
//                              &mut (2, _) => true,
//                              _ => false,
//                          };
//                          if matches {
//                              return Use(Arm2((from, msg)));
//                          }
//                      }
//                      _ => (),
//                  }
//              }
//              if m.tag == <usize as Message>::tag() {
//                  match ::erased_serde::deserialize::<Box<usize>>(&m.msg) {
//                      Ok(msg) => {
//                          let matches = match &mut *msg {
//                              &mut x if baz(x) => true,
//                              _ => false,
//                          };
//                          if matches {
//                              return Use(Arm3((from, msg)));
//                          }
//                      }
//                      _ => (),
//                  }
//              }
//              Skip(ReceivedMessage::Remote((from, m)))
//          },
//      }
//  })) {
//      Arm1(msg) => match *msg {
//          (_pid, (1, ref x)) => bar(x),
//          _ => unreachable!(),
//      },
//      Arm2(msg) => match *msg {
//          (_pid, (2, x)) => foobar(x),
//          _ => unreachable!(),
//      },
//      Arm3(msg) => match *msg {
//          (_pid, x) => quux(x),
//          _ => unreachable!(),
//      },
//  }

// Note: the match guards will be evaluated in an `async move` closure, hence
// it isn't possible to early-return from there, and every non-Copy local
// variable used in guards will be moved. In exchange, it is possible to call
// await!().
pub fn receive(input: TokenStream) -> TokenStream {
    // TODO: (B) Give nicer parsing errors, pinpointing the error, etc.
    let parsed = syn::parse2::<Receive>(input).expect(
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
        let name = gen_arm_ident(i);
        let ty = arm.ty.clone();
        quote!(#name((::erlust::Pid, Box<#ty>)))
    });
    let arms_def = quote!(
        enum MatchedArm {
            #(#names_and_types ,)*
        }
    );

    // Generate the inner matches
    let local_matches = parsed.arms.iter().cloned().enumerate().map(|(i, arm)| {
        if let Some(guard) = arm.guard {
            gen_local_match(i, arm.ty, arm.pat, quote!(if #guard))
        } else {
            let ignoring_pat = fold_pat(&mut PatIgnorer(), arm.pat);
            gen_local_match(i, arm.ty, ignoring_pat, quote!())
        }
    });

    // Generate the deserialize-attempt matches
    let remote_matches = parsed.arms.iter().cloned().enumerate().map(|(i, arm)| {
        if let Some(guard) = arm.guard {
            gen_remote_match(i, arm.ty, arm.pat, quote!(if #guard))
        } else {
            let ignoring_pat = fold_pat(&mut PatIgnorer(), arm.pat);
            gen_remote_match(i, arm.ty, ignoring_pat, quote!())
        }
    });

    // Generate the outer match's arms
    let execute_match_arms = parsed
        .arms
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, arm)| gen_execute_match_arm(i, arm.pat, arm.body));

    // TODO: (A) assert for each type it's a Message
    let res = quote! {
        #[allow(unused_variables)]
        {
            #arms_def

            match ::erlust::receive(async move |mut msg: ::erlust::ReceivedMessage| {
                match msg {
                    ::erlust::ReceivedMessage::Local((from, mut msg)) => {
                        #(#local_matches)*
                        ::erlust::ReceiveResult::Skip(
                            ::erlust::ReceivedMessage::Local((from, msg))
                        )
                    }
                    ::erlust::ReceivedMessage::Remote((from, m)) => {
                        #(#remote_matches)*
                        ::erlust::ReceiveResult::Skip(
                            ::erlust::ReceivedMessage::Remote((from, m))
                        )
                    }
                }
            }).await {
                #(#execute_match_arms)*
            }
        }
    };
    res.into()
}
