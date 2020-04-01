use crate::parser::ExtraHandler;
use crate::parser::OptionsMatch;

pub struct OptionsHelp {
    meta: Option<String>,
    msg: Option<String>,
}

impl OptionsHelp {
    pub(crate) fn to_pair<P>(&self, m: &OptionsMatch<P>) -> (String, String) {
        let lhs = match *m {
            OptionsMatch::Args(ref aliases, argct, _) => {
                let mut lhs = String::new();
                for (i, alias) in aliases.iter().enumerate() {
                    if i > 0 {
                        lhs.push_str("|")
                    }
                    lhs.push_str("-");
                    if alias.len() > 1 {
                        lhs.push_str("-");
                    }
                    lhs.push_str(alias);
                }
                if argct > 0 {
                    match self.meta {
                        Some(ref s) => {
                            lhs.push_str(" ");
                            lhs.push_str(s);
                        }
                        None => {
                            for _ in 0..argct {
                                lhs.push_str(" <arg>");
                            }
                        }
                    }
                }
                lhs
            },
            OptionsMatch::Extra(ExtraHandler::Soft(_)) => {
                match self.meta {
                    Some(ref s) => s.clone(),
                    None => "<arg>".to_string(),
                }
            },
            OptionsMatch::Extra(ExtraHandler::Hard(_)) => {
                match self.meta {
                    Some(ref s) => s.clone(),
                    None => "<args>".to_string(),
                }
            },
        };

        let rhs = self.msg.clone().unwrap_or_else(String::new);

        (lhs, rhs)
    }
}

// we'd love to use Into<String> but then rust thinks impls below may conflict if someone adds e.g.
// Into<String> for () which is of course impossible.  We use the usual escape hatch of a local
// trait.
pub trait ToOptionsHelpString: Into<String> {
}

impl ToOptionsHelpString for String {
}

impl ToOptionsHelpString for &str {
}

pub trait ToOptionsHelp {
    fn to_help(self) -> Option<OptionsHelp>;
}

impl ToOptionsHelp for () {
    fn to_help(self) -> Option<OptionsHelp> {
        Some(OptionsHelp {
            meta: None,
            msg: None,
        })
    }
}

impl<S: ToOptionsHelpString> ToOptionsHelp for S {
    fn to_help(self) -> Option<OptionsHelp> {
        Some(OptionsHelp {
            meta: None,
            msg: Some(self.into()),
        })
    }
}

impl<S1: ToOptionsHelpString, S2: ToOptionsHelpString> ToOptionsHelp for (S1, S2) {
    fn to_help(self) -> Option<OptionsHelp> {
        Some(OptionsHelp {
            meta: Some(self.0.into()),
            msg: Some(self.1.into()),
        })
    }
}

pub enum NoHelp {
}

impl ToOptionsHelp for Option<NoHelp> {
    fn to_help(self) -> Option<OptionsHelp> {
        None
    }
}
