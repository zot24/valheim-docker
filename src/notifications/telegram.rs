use crate::notifications::NotificationMessage;
use log::debug;
use serde::{Deserialize, Serialize};
use std::env;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";

pub fn is_telegram_api(webhook_url: &str) -> bool {
  webhook_url.starts_with(TELEGRAM_API_BASE)
}

#[derive(Deserialize, Serialize)]
pub struct TelegramAPISendMessageBody {
  chat_id: String,
  text: String,
}

impl TelegramAPISendMessageBody {
  pub fn new(event: &NotificationMessage) -> Self {
    let chat_id = env::var("TELEGRAM_CHAT_ID").unwrap();
    let payload = TelegramAPISendMessageBody {
      chat_id: chat_id,
      text: format!("{}: {}", String::from(&event.event_type.name), String::from(&event.event_message)),
    };
    debug!(
      "Telegram Payload: {}",
      serde_json::to_string(&payload).unwrap()
    );
    payload
  }
}

impl From<&NotificationMessage> for TelegramAPISendMessageBody {
  fn from(event: &NotificationMessage) -> Self {
    Self::new(event)
  }
}
