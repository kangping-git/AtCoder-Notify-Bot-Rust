# AtCoder Notify Bot

This project provides a Discord bot that notifies users about AtCoder contests and tracks rating information. The bot is written in Rust and uses MySQL for storage.

## Requirements
- Rust toolchain
- MySQL database
- Discord bot token
- AtCoder account credentials

## Configuration
Create a `config.env` file with the following environment variables:
```env
DISCORD_TOKEN="<Discord Token>"
ATCODER_USER="<AtCoder Username>"
ATCODER_PASS="<AtCoder Password>"
MYSQL_USER="<MySQL Username>"
MYSQL_PASS="<MySQL Password>"
MYSQL_DATABASE="<MySQL Database>"
MYSQL_HOST="<MySQL Host>"
MYSQL_PORT="<MySQL Port>"
PORT="<Web server port>"
```

## Usage
Build and run the bot with:
```bash
cargo run --release
```
The web server listens on the port specified by `PORT` and provides rating charts and other pages in the `static` directory.

## License
This project is licensed under the MIT License.
