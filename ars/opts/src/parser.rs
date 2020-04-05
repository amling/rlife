use ars_ds::pointer_rc::PointerRc;
use ars_validates::Validates;
use ars_validates::ValidationError;
use ars_validates::ValidationResult;
use std::rc::Rc;

use crate::help::OptionsHelp;
use crate::help::ToOptionsHelp;
use crate::trie::NameTrie;
use crate::trie::NameTrieResult;

type CbMany<P> = PointerRc<dyn Fn(&mut P, &[String]) -> ValidationResult<()>>;
type CbOne<P> = PointerRc<dyn Fn(&mut P, &str) -> ValidationResult<bool>>;

pub(crate) enum ExtraHandler<P> {
    Soft(CbOne<P>),
    Hard(CbMany<P>),
}

pub(crate) enum OptionsMatch<P> {
    Args(Vec<String>, usize, CbMany<P>),
    Extra(ExtraHandler<P>),
}

pub struct OptionsPile<P>(Vec<(OptionsMatch<P>, Option<OptionsHelp>)>);

impl<P: 'static> OptionsPile<P> {
    pub fn new() -> Self {
        OptionsPile(Vec::new())
    }

    pub fn match_single(&mut self, aliases: &[&str], f: impl Fn(&mut P, &str) -> ValidationResult<()> + 'static, help: impl ToOptionsHelp) {
        self.match_n(aliases, 1, move |p, a| f(p, &a[0]), help);
    }

    pub fn match_zero(&mut self, aliases: &[&str], f: impl Fn(&mut P) -> ValidationResult<()> + 'static, help: impl ToOptionsHelp) {
        self.match_n(aliases, 0, move |p, _a| f(p), help);
    }

    pub fn match_n(&mut self, aliases: &[&str], argct: usize, f: impl Fn(&mut P, &[String]) -> ValidationResult<()> + 'static, help: impl ToOptionsHelp) {
        self.0.push((OptionsMatch::Args(aliases.iter().map(|s| s.to_string()).collect(), argct, PointerRc(Rc::new(f))), help.to_help()));
    }

    pub fn match_extra_soft(&mut self, f: impl Fn(&mut P, &str) -> ValidationResult<bool> + 'static, help: impl ToOptionsHelp) {
        self.0.push((OptionsMatch::Extra(ExtraHandler::Soft(PointerRc(Rc::new(f)))), help.to_help()));
    }

    pub fn match_extra_hard(&mut self, f: impl Fn(&mut P, &[String]) -> ValidationResult<()> + 'static, help: impl ToOptionsHelp) {
        self.0.push((OptionsMatch::Extra(ExtraHandler::Hard(PointerRc(Rc::new(f)))), help.to_help()));
    }

    pub fn add(&mut self, mut other: OptionsPile<P>) {
        self.0.append(&mut other.0);
    }

    pub fn add_sub<P2: 'static>(&mut self, f: impl Fn(&mut P) -> &mut P2 + 'static, other: OptionsPile<P2>) {
        self.add(other.sub(f));
    }

    pub fn to_parser(&self) -> OptParser<P> {
        let mut opt = OptParser::default();
        for e in self.0.iter() {
            match e.0 {
                OptionsMatch::Args(ref aliases, argct, ref f) => {
                    for alias in aliases {
                        opt.named.insert(&alias, (argct, f.clone()));
                    }
                }
                OptionsMatch::Extra(ExtraHandler::Soft(ref h)) => {
                    opt.extra.push(ExtraHandler::Soft(h.clone()));
                }
                OptionsMatch::Extra(ExtraHandler::Hard(ref h)) => {
                    opt.extra.push(ExtraHandler::Hard(h.clone()));
                }
            }
        }
        opt
    }

    pub fn sub<P2>(self, f1: impl Fn(&mut P2) -> &mut P + 'static) -> OptionsPile<P2> {
        let f1 = Rc::new(f1);
        OptionsPile(self.0.into_iter().map(|e| {
            let f1 = f1.clone();
            (match e.0 {
                OptionsMatch::Args(aliases, argct, f) => OptionsMatch::Args(aliases, argct, PointerRc(Rc::new(move |p, a| (f.0)(f1(p), a)))),
                OptionsMatch::Extra(ExtraHandler::Soft(h)) => OptionsMatch::Extra(ExtraHandler::Soft(PointerRc(Rc::new(move |p, a| (h.0)(f1(p), a))))),
                OptionsMatch::Extra(ExtraHandler::Hard(h)) => OptionsMatch::Extra(ExtraHandler::Hard(PointerRc(Rc::new(move |p, a| (h.0)(f1(p), a))))),
            }, e.1)
        }).collect())
    }

    pub fn dump_help(&self) -> Vec<String> {
        let lines: Vec<_> = self.0.iter().filter_map(|e| {
            let (m, help) = e;
            help.as_ref().map(|help| help.to_pair(&m))
        }).collect();

        let width = lines.iter().map(|(lhs, _rhs)| lhs.len()).max().unwrap();

        lines.iter().map(|(lhs, rhs)| format!("{:width$}   {}", lhs, rhs, width = width)).collect()
    }
}



pub trait Optionsable {
    type Options: Default + Validates + 'static;

    fn options(opt: &mut OptionsPile<Self::Options>);

    fn new_options() -> OptionsPile<Self::Options> {
        let mut opt = OptionsPile::new();
        Self::options(&mut opt);
        opt
    }
}



pub struct OptParser<P> {
    named: NameTrie<(usize, CbMany<P>)>,
    extra: Vec<ExtraHandler<P>>,
}

impl<P> Default for OptParser<P> {
    fn default() -> Self {
        OptParser {
            named: NameTrie::default(),
            extra: Vec::default(),
        }
    }
}

fn name_from_arg(name: &str) -> Option<&str> {
    if name.starts_with("--") {
        return Some(&name[2..]);
    }
    if name.starts_with("-") {
        return Some(&name[1..]);
    }
    None
}

impl<P: 'static> OptParser<P> {
    pub fn parse_mut(&self, args: &[String], p: &mut P) -> ValidationResult<()> {
        let mut next_index = 0;
        let mut refuse_opt = false;
        'arg: loop {
            if next_index == args.len() {
                return Result::Ok(());
            }

            if !refuse_opt {
                if &args[next_index] == "--" {
                    refuse_opt = true;
                    next_index += 1;
                    continue 'arg;
                }

                if let Some(name) = name_from_arg(&args[next_index]) {
                    let (argct, f) = match self.named.get(name) {
                        NameTrieResult::None() => return ValidationError::message(format!("No such option {}", name)),
                        NameTrieResult::Unique(_, e) => e,
                        NameTrieResult::Collision(name1, name2) => return ValidationError::message(format!("Option {} is ambiguous (e.g.  {} and {})", name, name1, name2)),
                    };
                    let start = next_index + 1;
                    let end = start + argct;
                    if end > args.len() {
                        return ValidationError::message(format!("Not enough arguments for {}", args[next_index]));
                    }
                    (f.0)(p, &args[start..end]).map_err(|e| e.label(format!("While handling {:?}", &args[next_index..end])))?;
                    next_index = end;
                    continue;
                }
            }

            for extra in &self.extra {
                match extra {
                    ExtraHandler::Soft(f) => {
                        if (f.0)(p, &args[next_index]).map_err(|e| e.label(format!("While handling {:?}", &args[next_index..=next_index])))? {
                            next_index += 1;
                            continue 'arg;
                        }
                    }
                    ExtraHandler::Hard(f) => {
                        (f.0)(p, &args[next_index..]).map_err(|e| e.label(format!("While handling {:?}", &args[next_index..])))?;
                        next_index = args.len();
                        continue 'arg;
                    }
                }
            }

            return ValidationError::message(format!("Unexpected extra arguments: {:?}", &args[next_index..]));
        }
    }
}

impl<P: Default + 'static> OptParser<P> {
    pub fn parse(&self, args: &[String]) -> ValidationResult<P> {
        let mut p = P::default();
        self.parse_mut(args, &mut p)?;
        Result::Ok(p)
    }
}
