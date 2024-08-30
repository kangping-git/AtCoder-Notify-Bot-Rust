use crate::{Context, Error};

use super::owner::owner;
use crate::commands::role::role;
use crate::commands::set_ac_notify::set_ac_notify;
use crate::commands::set_language::set_language;

#[poise::command(prefix_command, slash_command, subcommands("set_language", "role", "owner", "set_ac_notify"))]
pub async fn server(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
