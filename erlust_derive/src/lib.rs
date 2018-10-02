#![feature(async_await, proc_macro_diagnostic)]
#![recursion_limit = "128"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

mod block_or_expr;
mod derive_message;
mod pat_ignorer;
mod receive;

use self::proc_macro::TokenStream;

#[proc_macro_derive(Message, attributes(erlust_tag))]
pub fn derive_message_macro(input: TokenStream) -> TokenStream {
    derive_message::derive_message(input.into()).into()
}

#[proc_macro]
pub fn receive(input: TokenStream) -> TokenStream {
    receive::receive(input.into()).into()
}

// TODO: (A) receive_box
