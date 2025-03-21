use teloxide::macros::BotCommands;

pub mod create;
pub mod open;
pub mod status;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display the node's network ID.")]
    NetworkId,
    #[command(
        description = "<password>, create (initiate) a fresh discord wallet protected by a password of your choice.",
        parse_with = "split"
    )]
    Create { password: String },
    #[command(description = "get the status of your wallet.")]
    Status,
    #[command(description = "<password>, open your wallet with the password you set.")]
    Open { password: String },
}
