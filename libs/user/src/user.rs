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
}

impl From<teloxide::types::User> for TipUser {
    fn from(user: teloxide::types::User) -> Self {
        TipUser {
            identifier: user.id.to_string(),
            wallet_identifier: format!("tg_{}", user.id.to_string()),
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
