use crate::{Context, Error};

use crate::commands::contests::contest;
use crate::commands::help::help;
use crate::commands::link_accounts::link_account;
use crate::commands::link_accounts::show_linked_account;
use crate::commands::link_accounts::unlink_account;
use crate::commands::rating::rating;
use crate::commands::register_accounts::delete_account;
use crate::commands::register_accounts::register_account;
use crate::commands::register_accounts::show_accounts;
use crate::commands::set_notification_contest::{set_notification_contest, unset_notification_contest};
use crate::commands::set_notification_submission::{set_notification_submission, unset_notification_submission};
use crate::commands::show_notification::show_notification;

#[poise::command(
    prefix_command,
    slash_command,
    subcommands(
        "help",
        "set_notification",
        "unset_notification",
        "show_notification",
        "link_account",
        "unlink_account",
        "show_linked_account",
        "delete_account",
        "show_accounts",
        "register_account",
        "contest",
        "rating"
    )
)]
pub async fn atcoder(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
#[poise::command(
    prefix_command,
    slash_command,
    rename = "set-notification",
    subcommands("set_notification_contest", "set_notification_submission")
)]
pub async fn set_notification(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    rename = "unset-notification",
    subcommands("unset_notification_contest", "unset_notification_submission")
)]
pub async fn unset_notification(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
