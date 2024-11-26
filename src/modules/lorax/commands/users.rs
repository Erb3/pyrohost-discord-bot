use crate::{
    modules::lorax::database::{LoraxEvent, LoraxStage},
    Context, Error,
};
use poise::{
    command,
    serenity_prelude::{
        ButtonStyle, ComponentInteractionDataKind, CreateActionRow, CreateButton,
        CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
        CreateSelectMenuKind, CreateSelectMenuOption,
    },
    CreateReply,
};

const RESERVED_TREES: [&str; 10] = [
    "maple", "sakura", "baobab", "sequoia", "oak", "pine", "palm", "willow", "cherry", "redwood",
];

#[command(slash_command, guild_only, ephemeral)]
pub async fn submit(
    ctx: Context<'_>,
    #[description = "Your awesome tree name"] name: String,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let guild_id = ctx.guild_id().unwrap().get();
    let user_id = ctx.author().id.get();

    let event = match ctx.data().dbs.lorax.get_event(guild_id).await {
        Some(event) => event,
        None => {
            ctx.say("🛑 Oops! There's no Lorax event happening right now.")
                .await?;
            return Ok(());
        }
    };

    if event.stage == LoraxStage::Voting {
        ctx.say("🗳️ Submission period has ended, but voting is open!\n💡 Use `/lorax vote` to pick your favorite tree name.").await?;
        return Ok(());
    }

    if event.stage != LoraxStage::Submission {
        ctx.say("🚫 Submissions are closed at the moment. Stay tuned for the next event!")
            .await?;
        return Ok(());
    }

    let name = name.to_lowercase().trim().to_string();

    if !is_valid_tree_name(&name) {
        ctx.say(
            "❌ Invalid tree name. Please ensure it is between 3 and 32 alphabetic characters.",
        )
        .await?;
        return Ok(());
    }

    if RESERVED_TREES.contains(&name.as_str()) || name == "lorax" {
        ctx.say("🌲 That tree name is reserved. Try coming up with something unique! 🍃")
            .await?;
        return Ok(());
    }

    if event.tree_submissions.values().any(|t| t == &name) {
        ctx.say("🌳 Someone already suggested that name! How about a different one?")
            .await?;
        return Ok(());
    }

    match ctx
        .data()
        .dbs
        .lorax
        .submit_tree(guild_id, name.clone(), user_id)
        .await
    {
        Ok((is_update, old_submission)) => {
            let msg = if is_update {
                format!(
                    "🔄 Updated your submission from \"**{}**\" to \"**{}**\"!\n⏳ Stay tuned for the voting phase.",
                    old_submission.unwrap_or_default(),
                    name
                )
            } else {
                format!(
                    "🌳 Your tree name \"**{}**\" has been submitted!\n⏳ Stay tuned for the voting phase.",
                    name
                )
            };
            ctx.say(msg).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Unable to submit: {}", e)).await?;
        }
    }

    Ok(())
}

fn is_valid_tree_name(name: &str) -> bool {
    let name = name.trim();
    let length = name.len();
    length >= 3 && length <= 32 && name.chars().all(|c| c.is_ascii_alphabetic())
}

#[command(slash_command, guild_only, ephemeral)]
pub async fn vote(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let guild_id = ctx.guild_id().unwrap().get();
    let user_id = ctx.author().id.get();

    let event = match ctx.data().dbs.lorax.get_event(guild_id).await {
        Some(event) => event,
        None => {
            ctx.say("❌ There is no active event at the moment!")
                .await?;
            return Ok(());
        }
    };

    if !is_voting_stage(&event.stage) {
        ctx.say("🚫 Voting is not active at the moment.").await?;
        return Ok(());
    }

    let mut trees = get_available_trees(&event, user_id);
    if trees.is_empty() {
        ctx.say("🤔 There's nothing to vote on yet. Wait for more submissions!")
            .await?;
        return Ok(());
    }

    trees.sort();

    let page_size = 25;
    let total_pages = (trees.len() as f32 / page_size as f32).ceil() as usize;
    let mut current_page = 0;

    let create_reply = |page: usize| {
        let mut components = vec![CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "vote_tree",
                CreateSelectMenuKind::String {
                    options: trees
                        [page * page_size..(page * page_size + page_size).min(trees.len())]
                        .iter()
                        .map(|tree| CreateSelectMenuOption::new(tree, tree))
                        .collect(),
                },
            )
            .placeholder("Choose wisely..."),
        )];

        if total_pages > 1 {
            components.push(CreateActionRow::Buttons(vec![
                CreateButton::new("prev_page")
                    .emoji('◀')
                    .style(ButtonStyle::Secondary)
                    .disabled(page == 0),
                CreateButton::new("next_page")
                    .emoji('▶')
                    .style(ButtonStyle::Secondary)
                    .disabled(page >= total_pages - 1),
            ]));
        }

        CreateReply::default()
            .content(format!(
                "🗳️ **Vote for your favorite tree name!** (Page {}/{})",
                page + 1,
                total_pages
            ))
            .components(components)
    };

    let create_update = |page: usize| {
        let mut components = vec![CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "vote_tree",
                CreateSelectMenuKind::String {
                    options: trees
                        [page * page_size..(page * page_size + page_size).min(trees.len())]
                        .iter()
                        .map(|tree| CreateSelectMenuOption::new(tree, tree))
                        .collect(),
                },
            )
            .placeholder("Choose wisely..."),
        )];

        if total_pages > 1 {
            components.push(CreateActionRow::Buttons(vec![
                CreateButton::new("prev_page")
                    .emoji('◀')
                    .style(ButtonStyle::Secondary)
                    .disabled(page == 0),
                CreateButton::new("next_page")
                    .emoji('▶')
                    .style(ButtonStyle::Secondary)
                    .disabled(page >= total_pages - 1),
            ]));
        }

        CreateInteractionResponseMessage::new()
            .content(format!(
                "🗳️ Pick your favorite tree name: (Page {}/{})",
                page + 1,
                total_pages
            ))
            .components(components)
    };

    let msg = ctx.send(create_reply(current_page)).await?;

    while let Some(interaction) = msg
        .message()
        .await?
        .await_component_interaction(ctx)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        match interaction.data.custom_id.as_str() {
            "prev_page" => {
                if current_page > 0 {
                    current_page -= 1;
                    interaction
                        .create_response(
                            &ctx.serenity_context().http,
                            CreateInteractionResponse::UpdateMessage(create_update(current_page)),
                        )
                        .await?;
                }
            }
            "next_page" => {
                if current_page < total_pages - 1 {
                    current_page += 1;
                    interaction
                        .create_response(
                            &ctx.serenity_context().http,
                            CreateInteractionResponse::UpdateMessage(create_update(current_page)),
                        )
                        .await?;
                }
            }
            "vote_tree" => {
                if let ComponentInteractionDataKind::StringSelect { values, .. } =
                    &interaction.data.kind
                {
                    let selected_tree = values.first().ok_or("No selection made")?;

                    match ctx
                        .data()
                        .dbs
                        .lorax
                        .write(|db| {
                            let event = db
                                .events
                                .get_mut(&guild_id)
                                .ok_or_else(|| "No active event".to_string())?;

                            if event.tree_votes.contains_key(&user_id) {
                                return Err("You've already voted!".to_string());
                            }

                            event.tree_votes.insert(user_id, selected_tree.to_string());
                            Ok(())
                        })
                        .await
                    {
                        Ok(_) => {
                            ctx.say("✅ Vote recorded!").await?;
                        }
                        Err(e) => {
                            ctx.say(format!("❌ Unable to cast vote: {}", e)).await?;
                        }
                    }
                } else {
                    return Err("Invalid interaction data kind".into());
                }
            }
            _ => {}
        }
    }

    ctx.say("⌛ Time's up! Feel free to `/vote` again anytime.")
        .await?;
    Ok(())
}

fn is_voting_stage(stage: &LoraxStage) -> bool {
    matches!(stage, LoraxStage::Voting | LoraxStage::Tiebreaker(_))
}

fn get_available_trees(event: &LoraxEvent, user_id: u64) -> Vec<String> {
    event
        .tree_submissions
        .iter()
        .filter(|(&submitter_id, _)| submitter_id != user_id)
        .map(|(_, tree)| tree.clone())
        .collect()
}
