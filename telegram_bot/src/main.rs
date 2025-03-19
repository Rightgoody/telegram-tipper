use core::tip_context::TipContext;

use spectre_wallet_core::rpc::ConnectOptions;
use spectre_wrpc_client::{
    Resolver, SpectreRpcClient, WrpcEncoding,
    prelude::{ConnectStrategy, NetworkId},
};
use std::{env, path::Path, str::FromStr, sync::Arc, time::Duration};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use core::utils::check_node_status;

use teloxide::{dispatching::dialogue::GetChatId, prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() {
    // load local .env or ignore if file doesn't exists
    match dotenvy::dotenv() {
        Ok(_) => println!("Environment variables loaded from .env"),
        Err(_) => println!("Not loading environement variables from .env"),
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let telegram_token = match env::var("TELEGRAM_TOKEN") {
        Ok(v) => v,
        Err(_) => panic!("TELEGRAM_TOKEN environment variable is missing."),
    };

    let spectre_network_str =
        env::var("SPECTRE_NETWORK").expect("SPECTRE_NETWORK environment variable is missing");

    let wallet_data_path_str =
        env::var("WALLET_DATA_PATH").expect("WALLET_DATA_PATH environment variable is missing");

    // RPC
    let forced_spectre_node: Option<String> = match env::var("FORCE_SPECTRE_NODE_ADDRESS") {
        Ok(v) => Some(v),
        Err(_) => None,
    };

    let resolver = match forced_spectre_node.clone() {
        Some(value) => Resolver::new(Some(vec![Arc::new(value)]), true), // tls
        _ => Resolver::default(),
    };

    let network_id = NetworkId::from_str(&spectre_network_str).unwrap();

    let wrpc_client = Arc::new(
        SpectreRpcClient::new(
            WrpcEncoding::Borsh,
            forced_spectre_node.as_deref(),
            Some(resolver.clone()),
            Some(network_id),
            None,
        )
        .unwrap(),
    );

    let connect_timeout = Duration::from_secs(5);

    match wrpc_client
        .connect(Some(ConnectOptions {
            url: forced_spectre_node.clone(),
            block_async_connect: true,
            connect_timeout: Some(connect_timeout),
            strategy: ConnectStrategy::Fallback,
            ..Default::default()
        }))
        .await
    {
        Ok(_) => info!(
            "Node {} is reachable, checking capabilities.",
            wrpc_client.ctl().descriptor().unwrap()
        ),
        Err(e) => {
            error!("Failed to connect to the node: {}", e);
            panic!("Connection failed: {}", e);
        }
    }

    match check_node_status(&wrpc_client).await {
        Ok(_) => {
            info!("Successfully completed client connection to the Spectre node!");
        }
        Err(error) => {
            error!("An error occurred: {}", error);
            std::process::exit(1);
        }
    }

    let wallet_data_path_buf = Path::new(&wallet_data_path_str).to_path_buf();

    let tip_context = TipContext::try_new_arc(
        resolver,
        NetworkId::from_str(&spectre_network_str).unwrap(),
        forced_spectre_node,
        wrpc_client,
        wallet_data_path_buf,
    );

    if let Err(e) = tip_context {
        panic!("{}", format!("Error while building tip context: {}", e));
    }

    let bot = Bot::new(telegram_token);

    let commands_handler = dptree::entry()
        .filter_command::<Command>()
        .endpoint(command_handler);

    let message_create_handler = Update::filter_message().branch(commands_handler);

    let main_handler = dptree::entry()
        .branch(message_create_handler)
        // default handler
        .branch(
            dptree::entry().endpoint(|bot: Bot, update: Update| async move {
                match update.chat_id() {
                    Some(chat_id) => {
                        bot.send_message(chat_id, format!("Unknown command or argument format is not correct.\nPlease use /help to see the list of available commands.")).await?;
                    }
                    None => {
                        warn!(
                            "default handler: Received update without chat_id: {:?}",
                            update
                        )
                    }
                }
                Ok(())
            }),
        );

    info!("Starting Telegram bot...");

    Dispatcher::builder(bot, main_handler)
        .dependencies(dptree::deps![tip_context.unwrap()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display the node's network ID.")]
    NetworkId,
}

async fn command_handler(
    bot: Bot,
    msg: Message,
    tip_context: Arc<TipContext>,
    cmd: Command,
) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::NetworkId => {
            let network_id = tip_context.network_id();
            bot.send_message(msg.chat.id, format!("Network ID: {}", network_id))
                .await?;
        }
    }
    Ok(())
}
