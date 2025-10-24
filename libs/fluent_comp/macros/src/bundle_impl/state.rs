use fluent_syntax::ast::{
    Attribute, Entry, Expression, InlineExpression, Message, Pattern, PatternElement, Resource,
    Term, VariantKey,
};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Ident;

use crate::model::BundleInput;

pub struct State<'t> {
    pub args: &'t BundleInput,
    pub bundle_ident: &'t Ident,
}

pub struct Variables<'t> {
    pub vars: Vec<Variable<'t>>,
}

#[derive(Clone, Copy)]
pub enum VariableKind {
    Display,
    Int,
    Str,
}

pub struct Variable<'t> {
    pub name: &'t str,
    pub kind: VariableKind,
}

pub struct ResSet<'t> {
    pub locale: &'t str,
    pub resource: &'t Resource<&'t str>,
}

#[derive(Clone)]
pub struct MessageSet<'t> {
    pub locale: &'t str,
    pub message: &'t Message<&'t str>,
}

#[derive(Clone)]
pub struct TermSet<'t> {
    pub locale: &'t str,
    pub term: &'t Term<&'t str>,
}

impl<'t> Variables<'t> {
    pub fn collect(pat: &Pattern<&'t str>) -> Self {
        fn inner_pattern<'t>(attr: &Pattern<&'t str>, vars: &mut Vec<Variable<'t>>) {
            for elem in &attr.elements {
                if let PatternElement::Placeable { expression } = elem {
                    inner_expr(expression, vars);
                }
            }
        }

        fn inner_expr<'t>(expr: &Expression<&'t str>, vars: &mut Vec<Variable<'t>>) {
            match expr {
                Expression::Select { selector, variants } => {
                    let mut kind = VariableKind::Int;
                    for variant in variants {
                        if !variant.default && matches!(variant.key, VariantKey::Identifier { .. })
                        {
                            kind = VariableKind::Str;
                        }

                        inner_pattern(&variant.value, vars);
                    }
                    inner_inline_expr(selector, kind, vars);
                },
                Expression::Inline(expr) => inner_inline_expr(expr, VariableKind::Display, vars),
            }
        }

        fn inner_inline_expr<'t>(
            expr: &InlineExpression<&'t str>,
            kind: VariableKind,
            vars: &mut Vec<Variable<'t>>,
        ) {
            match expr {
                InlineExpression::FunctionReference { .. }
                | InlineExpression::MessageReference { .. } => {}, // todo?
                InlineExpression::VariableReference { id } => add(vars, id.name, kind),
                InlineExpression::Placeable { expression } => inner_expr(expression, vars),
                _ => {},
            }
        }

        fn add<'t>(vars: &mut Vec<Variable<'t>>, name: &'t str, kind: VariableKind) {
            if let Some(var) = vars.iter_mut().find(|v| v.name == name) {
                var.kind.merge(kind);
            } else {
                vars.push(Variable { name, kind });
            }
        }

        let mut vars = Vec::new();
        inner_pattern(pat, &mut vars);
        Self { vars }
    }
}

impl VariableKind {
    pub fn merge(&mut self, other: Self) {
        match (&self, &other) {
            (_, Self::Str) | (Self::Display, _) => *self = other,
            _ => {},
        }
    }
}

impl ToTokens for VariableKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Display => quote::quote! { ::core::fmt::Display },
            Self::Int => quote::quote! { ::fluent_comp::FluentInt },
            Self::Str => quote::quote! { ::fluent_comp::FluentStr },
        });
    }
}

pub fn get_term<'t>(res: &'t Resource<&'t str>, id: &str) -> Option<&'t Term<&'t str>> {
    res.body.iter().find_map(|e| match e {
        Entry::Term(t) if t.id.name == id => Some(t),
        _ => None,
    })
}

pub fn get_message<'t>(res: &'t Resource<&'t str>, id: &str) -> Option<&'t Message<&'t str>> {
    res.body.iter().find_map(|e| match e {
        Entry::Message(m) if m.id.name == id => Some(m),
        _ => None,
    })
}

pub fn get_attribute<'t>(
    attributes: &'t [Attribute<&'t str>],
    id: &str,
) -> Option<&'t Attribute<&'t str>> {
    attributes.iter().find(|a| a.id.name == id)
}
