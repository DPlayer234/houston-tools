use fluent_syntax::ast::{
    Comment, Expression, InlineExpression, Pattern, PatternElement, Variant, VariantKey,
};
use proc_macro2::TokenStream;

use super::state::{MessageSet, State, Variables};
use crate::bundle_impl::state::{TermSet, get_attribute};
use crate::util::to_ident;

pub fn emit_header(state: &State<'_>) -> TokenStream {
    let State { args, bundle_ident } = state;
    let locales = &args.locales;

    quote::quote! {
        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::default::Default)]
        pub struct #bundle_ident {
            ____locale: #locales,
        }

        impl #bundle_ident {
            pub fn new(locale: #locales) -> Self {
                Self { ____locale: locale }
            }
        }
    }
}

pub fn emit_message(state: &State<'_>, sets: &[MessageSet<'_>]) -> TokenStream {
    let State { bundle_ident, args } = state;
    let locales = &args.locales;

    let (main, other) = sets.split_first().expect("should be at least one");

    let message_ident = to_ident(main.message.id.name);
    let attributes = main
        .message
        .attributes
        .iter()
        .map(|a| emit_message_attribute(state, a.id.name, sets));

    let comment = emit_comment(main.message.comment.as_ref(), main.message.value.as_ref());

    if let Some(pattern) = &main.message.value {
        let variables = Variables::collect(pattern);

        let var_names = variables.vars.iter().map(|s| to_ident(s.name));
        let var_tys = variables.vars.iter().map(|s| &s.kind);

        let mut fmt = TokenStream::new();
        let mut to_cow = TokenStream::new();

        for set in other {
            let Some(pattern) = &set.message.value else {
                continue;
            };

            let ident = to_ident(set.locale);
            let inner = emit_pattern_fmt(state, pattern);
            fmt.extend(quote::quote! { #locales::#ident => { #inner } });

            if let Some(inner) = emit_to_cow(pattern) {
                to_cow.extend(quote::quote! { #locales::#ident => { #inner } });
            }
        }

        let inner = emit_pattern_fmt(state, pattern);
        fmt.extend(quote::quote! { _ => { #inner } });

        let inner = emit_to_cow(pattern)
            .unwrap_or_else(|| quote::quote! { ::std::borrow::Cow::Owned(::std::string::ToString::to_string(self)) });
        to_cow.extend(quote::quote! { _ => { #inner } });

        quote::quote! {
            const _: () = {
                impl #bundle_ident {
                    #comment
                    pub fn #message_ident<'n>(self) -> ____MessageBuilder<'n, true> {
                        ____Message::builder().____locale(self.____locale)
                    }
                }

                #[derive(::fluent_comp::private::ConstBuilder)]
                pub struct ____Message<'a> {
                    #[builder(vis = "pub(self)")]
                    ____locale: #locales,
                    #(#var_names: &'a dyn #var_tys,)*
                    #[builder(vis = "pub(self)", default = ::core::marker::PhantomData)]
                    ____marker: ::core::marker::PhantomData<&'a ()>,
                }

                impl ____Message<'_> {
                    pub fn to_cow(&self) -> ::std::borrow::Cow<'static, ::core::primitive::str> {
                        match self.____locale { #to_cow }
                    }
                }

                impl<'a, 'b> ::core::convert::From<____Message<'a>> for ::std::borrow::Cow<'b, ::core::primitive::str> {
                    fn from(value: ____Message<'a>) -> Self {
                        value.to_cow()
                    }
                }

                impl ::core::fmt::Display for ____Message<'_> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        match self.____locale { #fmt }
                        ::core::fmt::Result::Ok(())
                    }
                }

                #(#attributes)*
            };
        }
    } else {
        quote::quote! {
            const _: () = {
                impl #bundle_ident {
                    #comment
                    pub fn #message_ident<'n>(self) -> ____MessageBuilder<'n, true> {
                        ____Message::builder().____locale(self.____locale)
                    }
                }

                #[derive(::fluent_comp::private::ConstBuilder)]
                pub struct ____Message<'a> {
                    #[builder(vis = "pub(self)")]
                    ____locale: #locales,
                    #[builder(vis = "pub(self)", default = ::core::marker::PhantomData)]
                    ____marker: ::core::marker::PhantomData<&'a ()>,
                }

                #(#attributes)*
            };
        }
    }
}

fn emit_message_attribute(state: &State<'_>, id: &str, sets: &[MessageSet<'_>]) -> TokenStream {
    let State { args, .. } = state;
    let locales = &args.locales;

    let (main, other) = sets.split_first().expect("should be at least one");
    let main_attr = get_attribute(&main.message.attributes, id).expect("should exist for main");

    let attr_ident = to_ident(id);
    let variables = Variables::collect(&main_attr.value);

    let var_names = variables.vars.iter().map(|s| to_ident(s.name));
    let var_tys = variables.vars.iter().map(|s| &s.kind);

    let mut fmt = TokenStream::new();
    let mut to_cow = TokenStream::new();

    for set in other {
        let Some(attr) = get_attribute(&set.message.attributes, id) else {
            continue;
        };

        let ident = to_ident(set.locale);
        let inner = emit_pattern_fmt(state, &attr.value);
        fmt.extend(quote::quote! { #locales::#ident => { #inner } });

        if let Some(inner) = emit_to_cow(&attr.value) {
            to_cow.extend(quote::quote! { #locales::#ident => { #inner } });
        }
    }

    let inner = emit_pattern_fmt(state, &main_attr.value);
    fmt.extend(quote::quote! { _ => { #inner } });

    let inner = emit_to_cow(&main_attr.value).unwrap_or_else(
        || quote::quote! { ::std::borrow::Cow::Owned(::std::string::ToString::to_string(self)) },
    );
    to_cow.extend(quote::quote! { _ => { #inner } });

    let comment = emit_comment(None, Some(&main_attr.value));

    quote::quote! {
        const _: () = {
            impl ____MessageBuilder<'_, true> {
                #comment
                pub fn #attr_ident<'n>(&self) -> ____AttributeBuilder<'n, true> {
                    let ____locale = unsafe { ::core::ptr::read(&raw const (*self.inner.inner.as_ptr()).____locale) };
                    ____Attribute::builder().____locale(____locale)
                }
            }

            #[derive(::fluent_comp::private::ConstBuilder)]
            pub struct ____Attribute<'a> {
                #[builder(vis = "pub(self)")]
                ____locale: #locales,
                #(#var_names: &'a dyn #var_tys,)*
                #[builder(vis = "pub(self)", default = ::core::marker::PhantomData)]
                ____marker: ::core::marker::PhantomData<&'a ()>,
            }

            impl ____Attribute<'_> {
                pub fn to_cow(&self) -> ::std::borrow::Cow<'static, ::core::primitive::str> {
                    match self.____locale { #to_cow }
                }
            }

            impl<'a, 'b> ::core::convert::From<____Attribute<'a>> for ::std::borrow::Cow<'b, ::core::primitive::str> {
                fn from(value: ____Attribute<'a>) -> Self {
                    value.to_cow()
                }
            }

            impl ::core::fmt::Display for ____Attribute<'_> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self.____locale { #fmt }
                    ::core::fmt::Result::Ok(())
                }
            }
        };
    }
}

pub fn emit_term(state: &State<'_>, sets: &[TermSet<'_>]) -> TokenStream {
    let State { bundle_ident, args } = state;
    let locales = &args.locales;

    let (main, other) = sets.split_first().expect("should be at least one");

    let message_ident = to_ident(main.term.id.name);
    let variables = Variables::collect(&main.term.value);

    let var_names = variables.vars.iter().map(|s| to_ident(s.name));
    let var_tys = variables.vars.iter().map(|s| &s.kind);

    let mut fmt = TokenStream::new();
    let mut to_cow = TokenStream::new();

    for set in other {
        let ident = to_ident(set.locale);
        let inner = emit_pattern_fmt(state, &set.term.value);
        fmt.extend(quote::quote! { #locales::#ident => { #inner } });

        if let Some(inner) = emit_to_cow(&set.term.value) {
            to_cow.extend(quote::quote! { #locales::#ident => { #inner } });
        }
    }

    let inner = emit_pattern_fmt(state, &main.term.value);
    fmt.extend(quote::quote! { _ => { #inner } });

    let inner = emit_to_cow(&main.term.value).unwrap_or_else(
        || quote::quote! { ::std::borrow::Cow::Owned(::std::string::ToString::to_string(self)) },
    );
    to_cow.extend(quote::quote! { _ => { #inner } });

    let attributes = main
        .term
        .attributes
        .iter()
        .map(|a| emit_term_attribute(state, a.id.name, sets));

    let comment = emit_comment(main.term.comment.as_ref(), Some(&main.term.value));

    quote::quote! {
        const _: () = {
            impl #bundle_ident {
                #comment
                fn #message_ident<'n>(self) -> ____TermBuilder<'n, true> {
                    ____Term::builder().____locale(self.____locale)
                }
            }

            #[derive(::fluent_comp::private::ConstBuilder)]
            pub struct ____Term<'a> {
                #[builder(vis = "pub(self)")]
                ____locale: #locales,
                #(
                    #[builder(default = &::fluent_comp::private::Unset)]
                    #var_names: &'a dyn #var_tys,
                )*
                #[builder(vis = "pub(self)", default = ::core::marker::PhantomData)]
                ____marker: ::core::marker::PhantomData<&'a ()>,
            }

            impl ____Term<'_> {
                pub fn to_cow(&self) -> ::std::borrow::Cow<'static, ::core::primitive::str> {
                    match self.____locale { #to_cow }
                }
            }

            impl<'a, 'b> ::core::convert::From<____Term<'a>> for ::std::borrow::Cow<'b, ::core::primitive::str> {
                fn from(value: ____Term<'a>) -> Self {
                    value.to_cow()
                }
            }

            impl ::core::fmt::Display for ____Term<'_> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self.____locale { #fmt }
                    ::core::fmt::Result::Ok(())
                }
            }

            #(#attributes)*
        };
    }
}

fn emit_term_attribute(state: &State<'_>, id: &str, sets: &[TermSet<'_>]) -> TokenStream {
    let State { args, .. } = state;
    let locales = &args.locales;

    let (main, other) = sets.split_first().expect("should be at least one");
    let main_attr = get_attribute(&main.term.attributes, id).expect("should exist for main");

    let attr_ident = to_ident(id);

    let mut fmt = TokenStream::new();
    for set in other {
        let Some(attr) = get_attribute(&set.term.attributes, id) else {
            continue;
        };

        let ident = to_ident(set.locale);
        let inner = emit_literal_fmt(&attr.value);
        fmt.extend(quote::quote! { #locales::#ident => { #inner } });
    }

    let inner = emit_literal_fmt(&main_attr.value);
    fmt.extend(quote::quote! { _ => { #inner } });

    quote::quote! {
        impl ____TermBuilder<'_, true> {
            pub fn #attr_ident<'n>(&self) -> &'static str {
                let ____locale = unsafe { ::core::ptr::read(&raw const (*self.inner.inner.as_ptr()).____locale) };
                match ____locale { #fmt }
            }
        }
    }
}

fn emit_pattern_fmt(state: &State<'_>, pattern: &Pattern<&str>) -> TokenStream {
    fn inner_pattern(state: &State<'_>, pattern: &Pattern<&str>, output: &mut TokenStream) {
        for element in &pattern.elements {
            match element {
                PatternElement::TextElement { value } => output.extend(quote::quote! {
                    f.write_str(#value)?;
                }),
                PatternElement::Placeable { expression } => inner_expr(state, expression, output),
            }
        }
    }

    fn inner_expr(state: &State<'_>, expr: &Expression<&str>, output: &mut TokenStream) {
        match expr {
            Expression::Select { selector, variants } => {
                inner_select(state, selector, variants, output)
            },
            Expression::Inline(inline) => inner_inline_expr(state, inline, output),
        }
    }

    fn inner_select(
        state: &State<'_>,
        selector: &InlineExpression<&str>,
        variants: &[Variant<&str>],
        output: &mut TokenStream,
    ) {
        let case = inline_expr_as_case(state, selector);
        let branches = variants.iter().map(|v| {
            let body = emit_pattern_fmt(state,&v.value, );
            match (v.default, &v.key) {
                (true, _) => quote::quote! { _ => { #body } },
                (false, VariantKey::NumberLiteral { value }) => {
                    match value.parse::<i8>() {
                        Ok(v) => quote::quote! { #v => { #body } },
                        Err(_) => quote::quote! { _ if { ::core::compile_error!("switch numbers must fit in i8") } => { #body } }
                    }
                },
                (false, VariantKey::Identifier { name }) => {
                    quote::quote! { #name => { #body } }
                }
            }
        });

        output.extend(quote::quote! {
            match #case {
                #(#branches),*
            }
        });
    }

    fn inner_inline_expr(
        state: &State<'_>,
        expr: &InlineExpression<&str>,
        output: &mut TokenStream,
    ) {
        let State { bundle_ident, .. } = state;
        match expr {
            InlineExpression::StringLiteral { value }
            | InlineExpression::NumberLiteral { value } => output.extend(quote::quote! {
                f.write_str(#value)?;
            }),
            InlineExpression::TermReference {
                id,
                attribute,
                arguments,
            } => {
                if attribute.is_some() {
                    output.extend(quote::quote! {
                        ::core::compile_error!("cannot specify term attribute in value position");
                    });
                }

                if arguments.as_ref().is_some_and(|a| !a.positional.is_empty()) {
                    output.extend(quote::quote! {
                        ::core::compile_error!("cannot positional term arguments");
                    });
                }

                let id = to_ident(id.name);
                let args = arguments
                    .as_ref()
                    .map(|a| a.named.iter())
                    .unwrap_or_default()
                    .map(|a| {
                        let id = to_ident(a.name.name);
                        let expr = inline_expr_as_argument(&a.value);
                        quote::quote! { .#id(#expr) }
                    });

                output.extend(quote::quote! {
                    ::core::fmt::Display::fmt(
                        &#bundle_ident::new(self.____locale)
                            .#id()
                            #(#args)*
                            .build(),
                        f,
                    )?;
                });
            },
            InlineExpression::VariableReference { id } => {
                let id = to_ident(id.name);
                output.extend(quote::quote! {
                    self.#id.fmt(f)?;
                })
            },
            InlineExpression::Placeable { expression } => {
                inner_expr(state, expression, output);
            },
            InlineExpression::MessageReference {
                id,
                attribute: None,
            } => {
                let id = to_ident(id.name);
                output.extend(quote::quote! {
                    ::core::fmt::Display::fmt(
                        &#bundle_ident::new(self.____locale).#id().build(),
                        f,
                    )?;
                });
            },
            InlineExpression::MessageReference {
                id,
                attribute: Some(attr),
            } => {
                let id = to_ident(id.name);
                let attr = to_ident(attr.name);
                output.extend(quote::quote! {
                    ::core::fmt::Display::fmt(&#bundle_ident::new(self.____locale).#id().#attr.build(), f)?;
                });
            },
            _ => output.extend(quote::quote! {
                ::core::compile_error!("unsupported expression type found");
            }),
        }
    }

    fn inline_expr_as_case(state: &State<'_>, expr: &InlineExpression<&str>) -> TokenStream {
        match expr {
            InlineExpression::TermReference {
                id,
                attribute: Some(attr),
                arguments: None,
            } => {
                let State { bundle_ident, .. } = state;
                let id = to_ident(id.name);
                let attr = to_ident(attr.name);

                quote::quote! {
                    #bundle_ident::new(self.____locale)
                        .#id()
                        .#attr()
                }
            },
            InlineExpression::VariableReference { id } => {
                let id = to_ident(id.name);
                quote::quote! { self.#id.to_switch_value() }
            },
            _ => quote::quote! {
                { ::core::compile_error!("unsupported case expression type found") }
            },
        }
    }

    fn inline_expr_as_argument(expr: &InlineExpression<&str>) -> TokenStream {
        match expr {
            InlineExpression::StringLiteral { value } => quote::quote! { &#value },
            InlineExpression::NumberLiteral { value } => value
                .parse::<isize>()
                .map(|i| quote::quote! { &#i })
                .unwrap_or_else(
                    |_| quote::quote! { { ::core::compile_error!("unsupported integer literal") } },
                ),
            InlineExpression::VariableReference { id } => {
                let id = to_ident(id.name);
                quote::quote! { &self.#id }
            },
            _ => quote::quote! {
                { ::core::compile_error!("unsupported argument expression type found") }
            },
        }
    }

    let mut output = TokenStream::new();
    inner_pattern(state, pattern, &mut output);
    output
}

fn pattern_as_str<'t>(pattern: &Pattern<&'t str>) -> Option<&'t str> {
    if let [single] = pattern.elements.as_slice() {
        pattern_element_as_str(single)
    } else {
        None
    }
}

fn pattern_element_as_str<'t>(element: &PatternElement<&'t str>) -> Option<&'t str> {
    if let PatternElement::TextElement { value }
    | PatternElement::Placeable {
        expression: Expression::Inline(InlineExpression::StringLiteral { value }),
    } = element
    {
        Some(value)
    } else {
        None
    }
}

fn emit_literal_fmt(pattern: &Pattern<&str>) -> TokenStream {
    if let Some(value) = pattern_as_str(pattern) {
        quote::quote! { #value }
    } else {
        quote::quote! {
            { ::core::compile_error!("unsupported term attribute value") }
        }
    }
}

fn emit_to_cow(pattern: &Pattern<&str>) -> Option<TokenStream> {
    pattern_as_str(pattern).map(|value| quote::quote! { ::std::borrow::Cow::Borrowed(#value) })
}

fn emit_comment(comment: Option<&Comment<&str>>, pattern: Option<&Pattern<&str>>) -> TokenStream {
    let mut output = TokenStream::new();

    if let Some(pattern) = pattern {
        let mut like = String::new();
        like.push_str("Creates a localized message like: \"");

        for element in &pattern.elements {
            if let Some(value) = pattern_element_as_str(element) {
                like.push_str(value);
            } else {
                like.push_str("`{..}`");
            }
        }

        like.push('"');

        output.extend(quote::quote! {
            #[doc = #like]
            #[doc = ""]
        });
    }

    if let Some(comment) = comment {
        for part in &comment.content {
            output.extend(quote::quote! {
                #[doc = #part]
            });
        }
    }

    output
}
