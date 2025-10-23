use std::fs;
use std::path::Path;

use fluent_syntax::ast::Entry;
use proc_macro2::TokenStream;
use syn::ItemStruct;

use crate::model::BundleInput;
use crate::util::get_manifest_dir;

mod emit;
mod state;

pub fn entry_point(args: BundleInput, input: ItemStruct) -> darling::Result<TokenStream> {
    use state::*;

    let dir = get_manifest_dir();

    let contents = args
        .resources
        .iter()
        .map(|(k, v)| {
            let path = Path::new(&dir).join(v);
            fs::read_to_string(path)
                .map(|f| (k.as_str(), f))
                .map_err(|err| {
                    darling::Error::custom(format!("cannot read resource at `{v}`: {err}",))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let asts = contents
        .iter()
        .map(|(k, f)| {
            fluent_syntax::parser::parse(f.as_str())
                .map(|f| (*k, f))
                .map_err(|err| {
                    darling::Error::custom(format!(
                        "cannot parse resource at `{k}`: {}",
                        err.1.first().expect("should be at least 1 error")
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut sets = asts
        .iter()
        .map(|a| ResSet {
            locale: a.0,
            resource: &a.1,
        })
        .collect::<Vec<_>>();

    sets.sort_unstable_by_key(|s| (s.locale != args.default, s.locale));

    let (default_set, other_sets) = sets.split_first().expect("should always have one");

    let term_sets = default_set.resource.body.iter().filter_map(|e| match e {
        Entry::Term(t) => Some(TermSet {
            locale: default_set.locale,
            term: t,
        }),
        _ => None,
    });

    let message_sets = default_set.resource.body.iter().filter_map(|e| match e {
        Entry::Message(m) => Some(MessageSet {
            locale: default_set.locale,
            message: m,
        }),
        _ => None,
    });

    let state = State {
        args: &args,
        bundle_ident: &input.ident,
    };

    let mut output = emit::emit_header(&state);

    for main in term_sets {
        let mut set = vec![main.clone()];
        for other in other_sets {
            if let Some(t) = get_term(other.resource, main.term.id.name) {
                set.push(TermSet {
                    locale: other.locale,
                    term: t,
                });
            }
        }

        output.extend(emit::emit_term(&state, &set));
    }

    for main in message_sets {
        let mut set = vec![main.clone()];
        for other in other_sets {
            if let Some(m) = get_message(other.resource, main.message.id.name) {
                set.push(MessageSet {
                    locale: other.locale,
                    message: m,
                });
            }
        }

        output.extend(emit::emit_message(&state, &set));
    }

    Ok(output)
}
