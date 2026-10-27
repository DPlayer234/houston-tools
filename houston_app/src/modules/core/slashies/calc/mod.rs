use crate::slashies::prelude::*;

mod ops;
mod parse;

/// Evaluates a mathematical equation. Warning: Floating point math.
#[chat_command(contexts(Guild, BotDm, PrivateChannel), integration_types(Guild, User))]
pub async fn calc(
    ctx: Context<'_>,
    /// The expression to evaluate.
    #[max_length = 3000]
    expression: &str,
    /// Whether to show the response only to yourself.
    mut ephemeral: Option<bool>,
) -> anyhow::Result<()> {
    let expression = expression.to_ascii_lowercase();

    let (content, color) = match parse::evaluate(&expression) {
        Ok(result) => (
            format!("{expression} = **{result}**"),
            ctx.data_ref().config().embed_color,
        ),
        Err(err) => {
            ephemeral = Some(true);
            (format!("```\n{err}\n```"), ERROR_EMBED_COLOR)
        },
    };

    let components = components_array![CreateTextDisplay::new(content)];
    let components = components_array![CreateContainer::new(&components).accent_color(color)];

    let reply = create_reply(ephemeral)
        .components_v2(&components)
        .allowed_mentions(CreateAllowedMentions::new());

    ctx.send(reply).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse::evaluate;

    macro_rules! is_correct {
        ($math:literal, $result:literal) => {{
            const MIN: f64 = $result - 0.001;
            const MAX: f64 = $result + 0.001;
            let text = $math;
            let res = evaluate(text);
            assert!(
                matches!(res, Ok(MIN..=MAX)),
                "`{text:?}` not in `{MIN}..={MAX}`, was {res:?}"
            );
        }};
    }

    #[test]
    fn success() {
        is_correct!("-4.5", -4.5);
        is_correct!("1 + 2 * 3", 7.0);
        is_correct!("1 + min(2) * 3", 7.0);
        is_correct!("sin(pi)", 0.0);
        is_correct!("min(2, max(-3, +5, 2), 21) * log(10, 100)", 4.0);
        is_correct!("min()", 0.0);
        is_correct!("1--2", 3.0);
    }
}
