use crate::{Context, Error};
use poise::{
    command,
    serenity_prelude::{self as serenity, Mentionable},
};

/// Configure Lorax settings for your server
#[command(
    slash_command,
    guild_only,
    required_permissions = "MANAGE_GUILD",
    subcommands("channel", "roles", "durations", "view")
)]
pub async fn config(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the announcement channel
#[command(slash_command, guild_only)]
pub async fn channel(
    ctx: Context<'_>,
    #[description = "Channel for Lorax announcements"] channel: serenity::Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get();

    ctx.data()
        .dbs
        .lorax
        .write(|db| {
            let settings = db.settings.entry(guild_id).or_default();
            settings.lorax_channel = Some(channel.id().get());
            Ok(())
        })
        .await?;

    ctx.say(format!(
        "✅ Lorax announcements will be in {}!",
        channel.mention()
    ))
    .await?;
    Ok(())
}

/// Configure role settings
#[command(slash_command, guild_only)]
pub async fn roles(
    ctx: Context<'_>,
    #[description = "Role to mention for events"] event_role: Option<serenity::Role>,
    #[description = "Role awarded to winners"] winner_role: Option<serenity::Role>,
    #[description = "Role for previous winners"] alumni_role: Option<serenity::Role>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get();

    let guild = ctx.guild().map(|g| g.clone());
    let bot_top_role = if let Some(guild) = guild.as_ref() {
        if let Some(bot_member) = guild.members.get(&ctx.framework().bot_id) {
            bot_member
                .roles
                .iter()
                .filter_map(|r| guild.roles.get(r))
                .max_by_key(|r| r.position)
        } else {
            None
        }
    } else {
        None
    };

    let roles_to_validate: Vec<_> = [&event_role, &winner_role, &alumni_role]
        .iter()
        .filter_map(|r| r.as_ref())
        .collect();

    if let Some(top_role) = bot_top_role {
        for role in &roles_to_validate {
            if role.position >= top_role.position {
                ctx.say("One or more roles are positioned higher than the bot's highest role.")
                    .await?;
                return Ok(());
            }
        }
    }

    let winner_role_exists = winner_role.is_some();
    let alumni_role_exists = alumni_role.is_some();

    ctx.data()
        .dbs
        .lorax
        .write(|db| {
            let settings = db.settings.entry(guild_id).or_default();
            if let Some(role) = event_role {
                settings.lorax_role = Some(role.id.get());
            }
            if let Some(role) = winner_role {
                settings.winner_role = Some(role.id.get());
            }
            if let Some(role) = alumni_role {
                settings.alumni_role = Some(role.id.get());
            }
            Ok(())
        })
        .await?;

    let mut response = "✅ Roles updated successfully!".to_string();

    if winner_role_exists && !alumni_role_exists {
        response.push_str("\n⚠️ Warning: Winner role is set but no alumni role is configured. Previous winners will lose their status.");
    }

    ctx.say(response).await?;
    Ok(())
}

/// Set event phase durations
#[command(slash_command, guild_only)]
pub async fn durations(
    ctx: Context<'_>,
    #[description = "Minutes for submissions"] submission: Option<u64>,
    #[description = "Minutes for voting"] voting: Option<u64>,
    #[description = "Minutes for tiebreakers"] tiebreaker: Option<u64>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get();

    ctx.data()
        .dbs
        .lorax
        .write(|db| {
            let settings = db.settings.entry(guild_id).or_default();
            if let Some(mins) = submission {
                settings.submission_duration = mins;
            }
            if let Some(mins) = voting {
                settings.voting_duration = mins;
            }
            if let Some(mins) = tiebreaker {
                settings.tiebreaker_duration = mins;
            }
            Ok(())
        })
        .await?;

    ctx.say("⏱️ Durations updated!").await?;
    Ok(())
}

/// View current Lorax settings
#[command(slash_command, guild_only)]
pub async fn view(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get();

    let settings = ctx
        .data()
        .dbs
        .lorax
        .get_settings(guild_id)
        .await
        .unwrap_or_default();

    let msg = format!(
        "⚙️ **Lorax Settings**\n\
        📢 **Channel:** {}\n\
        🎉 **Event Role:** {}\n\
        🏆 **Winner Role:** {}\n\
        🏅 **Alumni Role:** {}\n\
        ⏳ **Submission Duration:** {} minutes\n\
        ⏳ **Voting Duration:** {} minutes\n\
        ⏳ **Tiebreaker Duration:** {} minutes",
        settings
            .lorax_channel
            .map_or("Not set".into(), |id| format!("<#{}>", id)),
        settings
            .lorax_role
            .map_or("Not set".into(), |id| format!("<@&{}>", id)),
        settings
            .winner_role
            .map_or("Not set".into(), |id| format!("<@&{}>", id)),
        settings
            .alumni_role
            .map_or("Not set".into(), |id| format!("<@&{}>", id)),
        settings.submission_duration,
        settings.voting_duration,
        settings.tiebreaker_duration
    );

    ctx.say(msg).await?;
    Ok(())
}