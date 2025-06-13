# AtCoder Notify Bot Wiki

## Overview
AtCoder Notify Bot is a Discord bot written in Rust. It scrapes data from the AtCoder website to provide contest notifications and rating information. The bot also serves a web page for graphs and statistics.

## Setup
1. Prepare a MySQL database and a Discord bot token.
2. Create a `config.env` file in the project root with the following variables:
   ```env
   DISCORD_TOKEN="<Your Discord token>"
   ATCODER_USER="<AtCoder username>"
   ATCODER_PASS="<AtCoder password>"
   MYSQL_USER="<MySQL username>"
   MYSQL_PASS="<MySQL password>"
   MYSQL_DATABASE="<MySQL database>"
   MYSQL_HOST="<MySQL host>"
   MYSQL_PORT="<MySQL port>"
   PORT="<Web server port>"
   ```

## Running the Bot
Execute the following command to build and start the bot:
```bash
cargo run --release
```
The bot will connect to Discord and start the web server on the port specified in `config.env`.

## Features
- Contest and submission notifications
- Display of user ratings and charts
- Slash commands for managing accounts and server settings
