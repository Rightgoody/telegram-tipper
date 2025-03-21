use teloxide::{
    Bot, RequestError, payloads::SendMessageSetters, prelude::Requester, types::ParseMode,
};

pub struct TipUser {
    identifier: String,
    wallet_identifier: String,
}

impl TipUser {
    pub fn wallet_identifier(&self) -> &str {
        self.wallet_identifier.as_str()
    }
    pub fn identifier(&self) -> String {
        self.identifier.clone()
    }
    pub async fn send_telegram_message<T: Into<String>>(
        &self,
        bot: &Bot,
        message: T,
    ) -> Result<(), RequestError> {
        // replace every '.', '_' with '\\.' to escape markdown
        let message = message.into().replace('.', "\\.").replace('_', "\\_");

        bot.send_message(self.identifier(), message)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        Ok(())
    }
}

impl From<teloxide::types::User> for TipUser {
    fn from(user: teloxide::types::User) -> Self {
        TipUser {
            identifier: user.id.to_string(),
            wallet_identifier: format!("tg-{}", user.username.unwrap().to_lowercase()),
        }
    }
}

/// no prefix because it's the default user type for discord bots
impl From<serenity::model::user::User> for TipUser {
    fn from(user: serenity::model::user::User) -> Self {
        TipUser {
            identifier: user.id.to_string(),
            wallet_identifier: user.id.to_string(),
        }
    }
}
