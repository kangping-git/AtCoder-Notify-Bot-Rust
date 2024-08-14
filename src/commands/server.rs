use crate::{Context, Error};

use crate::commands::set_language::set_language;

#[poise::command(prefix_command, slash_command, subcommands("set_language"))]
pub async fn server(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
