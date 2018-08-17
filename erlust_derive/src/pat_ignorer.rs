use proc_macro2::Span;
use syn::{fold::Fold, token::Underscore, Pat, PatWild};

// Transforms all potential moves into _ ignorers
pub struct PatIgnorer();

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
