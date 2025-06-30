use utils::text::WriteStr as _;

use crate::buttons::prelude::*;
use crate::helper::discord::components::components;
use crate::helper::discord::{id_as_u64, unicode_emoji};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    states: [State; 2],
    action: Option<Choice>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct State {
    #[serde(with = "id_as_u64")]
    user: UserId,
    choice: Option<Choice>,
}

utils::impl_debug!(struct View: { states, .. });
utils::impl_debug!(struct State: { user, .. });

impl State {
    fn new(user: UserId) -> Self {
        Self { user, choice: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Choice {
    Rock,
    Paper,
    Scissors,
}

impl Choice {
    fn emoji(self) -> ReactionType {
        match self {
            Self::Rock => unicode_emoji("\u{1FAA8}"),
            Self::Paper => unicode_emoji("\u{1F4F0}"),
            Self::Scissors => unicode_emoji("✂️"),
        }
    }
}

enum Ready {
    Winner(UserId),
    Draw,
}

impl View {
    pub fn new(players: [UserId; 2]) -> Self {
        Self {
            states: players.map(State::new),
            action: None,
        }
    }

    fn state_mut(&mut self, user: UserId) -> Option<&mut State> {
        self.states.iter_mut().find(|s| s.user == user)
    }

    fn ready(&self) -> Option<Ready> {
        let c0 = self.states[0].choice?;
        let c1 = self.states[1].choice?;

        Some(match (c0, c1) {
            (Choice::Rock, Choice::Rock)
            | (Choice::Paper, Choice::Paper)
            | (Choice::Scissors, Choice::Scissors) => Ready::Draw,

            (Choice::Rock, Choice::Scissors)
            | (Choice::Paper, Choice::Rock)
            | (Choice::Scissors, Choice::Paper) => Ready::Winner(self.states[0].user),

            (Choice::Rock, Choice::Paper)
            | (Choice::Paper, Choice::Scissors)
            | (Choice::Scissors, Choice::Rock) => Ready::Winner(self.states[1].user),
        })
    }

    pub fn create_next_reply<'new>(mut self, data: &HBotData) -> CreateReply<'new> {
        fn hidden_label(this: Option<Choice>) -> &'static str {
            match this {
                Some(_) => "\u{2757} ",
                None => "",
            }
        }

        let description = format!(
            "{} {}VS {}{}",
            self.states[0].user.mention(),
            hidden_label(self.states[0].choice),
            hidden_label(self.states[1].choice),
            self.states[1].user.mention(),
        );

        let buttons = CreateActionRow::buttons(vec![
            self.new_action_button(Choice::Rock).label("Rock"),
            self.new_action_button(Choice::Paper).label("Paper"),
            self.new_action_button(Choice::Scissors).label("Scissors"),
        ]);

        let components = components![
            CreateContainer::new(components![
                description,
                CreateSeparator::new(true),
                buttons,
            ])
            .accent_color(data.config().embed_color)
        ];

        CreateReply::new()
            .components_v2(components)
            .allowed_mentions(CreateAllowedMentions::new())
    }

    fn create_ready_reply<'new>(self, data: &HBotData, ready: Ready) -> CreateReply<'new> {
        let mut description = String::with_capacity(64);
        match ready {
            Ready::Winner(user) => writeln!(description, "## {} wins!", user.mention()),
            Ready::Draw => writeln!(description, "## Draw!"),
        }

        let match_icon = match ready {
            Ready::Winner(user) if user == self.states[0].user => ">",
            Ready::Winner(_) => "<",
            Ready::Draw => "=",
        };

        writeln!(
            description,
            "{} {} **{}** {} {}",
            self.states[0].user.mention(),
            self.states[0].choice.unwrap_or(Choice::Rock).emoji(),
            match_icon,
            self.states[1].choice.unwrap_or(Choice::Rock).emoji(),
            self.states[1].user.mention(),
        );

        let components = components![
            CreateContainer::new(components![description]).accent_color(data.config().embed_color)
        ];

        CreateReply::new()
            .components_v2(components)
            .allowed_mentions(CreateAllowedMentions::new())
    }

    fn new_action_button<'new>(&mut self, choice: Choice) -> CreateButton<'new> {
        let custom_id = self.to_custom_id_with(|s| &mut s.action, Some(choice));
        CreateButton::new(custom_id).emoji(choice.emoji())
    }
}

button_value!(View, 19);
impl ButtonReply for View {
    async fn reply(mut self, ctx: ButtonContext<'_>) -> Result {
        let action = self.action;
        let state = self
            .state_mut(ctx.interaction.user.id)
            .ok_or(HArgError::new_const("You weren't invited to this round."))?;

        state.choice = action;

        let reply = if let Some(ready) = self.ready() {
            self.create_ready_reply(ctx.data, ready)
        } else {
            self.create_next_reply(ctx.data)
        };
        ctx.edit(reply.into()).await
    }
}
