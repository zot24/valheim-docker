use std::env;

use chrono::prelude::*;
use inflections::case::{to_constant_case, to_title_case};
use log::{debug, error, info, warn};
use reqwest::blocking::RequestBuilder;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::notifications::discord::{is_discord_webhook, DiscordWebHookBody};
use crate::notifications::telegram::{is_telegram_api, TelegramAPISendMessageBody};
use crate::notifications::enums::event_status::EventStatus;
use crate::notifications::enums::notification_event::{EventType, NotificationEvent};
use crate::utils::environment::fetch_var;
use reqwest::Url;

mod discord;
mod telegram;
pub mod enums;

pub const WEBHOOK_URL: &str = "WEBHOOK_URL";

#[derive(Deserialize, Serialize)]
pub struct NotificationMessage {
  event_type: EventType,
  event_message: String,
  timestamp: String,
}

fn fetch_webhook_url() -> String {
  fetch_var(WEBHOOK_URL, "")
    .trim_start_matches('"')
    .trim_end_matches('"')
    .to_string()
}

fn is_webhook_enabled() -> bool {
  let url = fetch_webhook_url();
  if !url.is_empty() {
    debug!("Webhook Url found!: {}", url);
    let is_valid = Url::parse(url.as_str()).is_ok();
    if !is_valid {
      warn!(
        "Webhook provided but does not look valid!! Is this right? {}",
        url
      )
    }
    return is_valid;
  }
  false
}

fn parse_webhook_env_var(event_type: EventType) -> String {
  if event_type.name.to_lowercase().eq("broadcast") {
    to_constant_case(format!("WEBHOOK_{}_MESSAGE", event_type.name).as_str())
  } else {
    to_constant_case(format!("WEBHOOK_{}_{}_MESSAGE", event_type.name, event_type.status).as_str())
  }
}

impl NotificationEvent {
  fn create_notification_message(&self) -> NotificationMessage {
    NotificationMessage {
      event_type: self.to_event_type(),
      event_message: format!(
        "Server Status: {}",
        to_title_case(self.to_string().as_str())
      ),
      timestamp: Local::now().to_rfc3339(),
    }
  }
  fn handle_request(&self, request: RequestBuilder) {
    let response = request.send();
    if let Ok(parsed_response) = response {
      let response_status = parsed_response.status();
      let response_message = parsed_response.text().unwrap();
      match response_status.as_u16() {
        200 | 204 | 201 => info!("[{}]: Webhook message sent successfully!", self),
        _ => error!("Request failed! {}, {}", response_status, response_message),
      }
    } else {
      error!(
        "[{}]: Error with webhook! Status {}",
        self,
        response
          .err()
          .unwrap()
          .status()
          .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
          .as_str()
      );
    }
  }
  fn build_request(&self, webhook_url: &str) -> RequestBuilder {
    let client = reqwest::blocking::Client::new();
    debug!("Webhook URL: {}", webhook_url);
    client.post(webhook_url)
  }
  pub fn send_custom_notification(&self, webhook_url: &str, message: &str) {
    let mut notification = self.create_notification_message();
    notification.event_message = message.to_string();
    debug!("Webhook enabled, sending notification {}", self.to_string());

    let mut req = self.build_request(webhook_url);
    req = if is_discord_webhook(webhook_url) {
      info!("Sending discord notification <3");
      req.json(&DiscordWebHookBody::from(&notification))
    } else if is_telegram_api(webhook_url) {
      info!("Sending telegram notification <3");
      req.json(&TelegramAPISendMessageBody::from(&notification))
    }else {
      debug!(
        "Webhook Payload: {}",
        serde_json::to_string(&notification).unwrap()
      );
      req.json(&notification)
    };
    self.handle_request(req);
  }
  pub fn send_notification(&self) {
    if is_webhook_enabled() {
      debug!("Webhook found! Starting notification process...");
      let event = self.create_notification_message();
      let env_var_name = parse_webhook_env_var(event.event_type);
      let notification_message = env::var(env_var_name).unwrap_or(event.event_message);
      self.send_custom_notification(fetch_webhook_url().as_str(), notification_message.as_str());
    } else {
      debug!("Skipping notification, no webhook supplied!");
    }
  }
}

#[cfg(test)]
mod webhook_tests {
  use super::*;
  use serial_test::serial;
  use std::env::{remove_var, set_var};

  #[test]
  #[serial]
  fn is_webhook_enabled_found_var_valid_url() {
    set_var("WEBHOOK_URL", "http://127.0.0.1:3000/dummy-url");
    assert_eq!(is_webhook_enabled(), true);
  }

  #[test]
  #[serial]
  fn is_webhook_enabled_found_var_invalid_url() {
    set_var("WEBHOOK_URL", "LOCALHOST");
    assert_eq!(is_webhook_enabled(), false);
  }

  #[test]
  #[serial]
  fn is_webhook_enabled_not_found_var() {
    remove_var("WEBHOOK_URL");
    assert_eq!(is_webhook_enabled(), false);
  }

  #[test]
  #[serial]
  fn is_webhook_enabled_empty_var() {
    set_var("WEBHOOK_URL", "");
    assert_eq!(is_webhook_enabled(), false);
  }
}

#[cfg(test)]
mod enum_tests {
  use inflections::case::to_title_case;

  use super::*;
  use crate::notifications::enums::event_status::EventStatus;
  use crate::notifications::NotificationEvent::Broadcast;

  #[test]
  fn parse_enum_as_string() {
    assert_eq!(to_title_case(Broadcast.to_string().as_str()), "Broadcast");
  }

  #[test]
  fn parse_enum_create_notification() {
    let event = NotificationEvent::Stop(EventStatus::Running);
    let notification = event.create_notification_message();
    assert_eq!(
      format!(
        "{} {}",
        notification.event_type.name, notification.event_type.status
      ),
      event.to_string()
    );
    assert!(notification.event_message.contains(&event.to_string()));
  }
}
