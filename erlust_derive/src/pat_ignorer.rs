use syn::{fold::Fold, Pat, PatWild};

// Transforms all potential moves into _ ignorers
pub struct PatIgnorer();

impl Fold for PatIgnorer {
    fn fold_pat(&mut self, p: Pat) -> Pat {
        use self::Pat::*;
        match p {
            Ident(p) => match p.subpat {
                Some((_at, subpat)) => self.fold_pat(*subpat),
                None => Wild(PatWild {
                    attrs: Vec::new(),
                    underscore_token: Default::default(),
                }),
            },
            p => p,
        }
    }
}
