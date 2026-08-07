//! Teloxide long-polling runtime.

use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Instant,
};

use routerbot_app::{AppError, CommandService};
use routerbot_core::{ActionRisk, BotCommand};
use teloxide::{
    dispatching::{UpdateFilterExt, UpdateHandler},
    payloads::{AnswerCallbackQuerySetters, EditMessageReplyMarkupSetters, SendMessageSetters},
    prelude::{Bot, CallbackQuery, Dispatcher, Message, Requester},
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me, MessageId, Update},
};

use crate::{
    TelegramAccessPolicy, TelegramCommand,
    confirmation::{ConfirmationKey, ConfirmationResolution, ConfirmationStore},
    format::{format_app_error, format_command_response, format_confirmation},
};

const CONFIRM_CALLBACK: &str = "routerbot:confirm";
const CANCEL_CALLBACK: &str = "routerbot:cancel";
const ACCESS_DENIED: &str = "Access denied.";

type HandlerError = Box<dyn Error + Send + Sync>;
type HandlerResult = Result<(), HandlerError>;

/// Telegram long-polling adapter for Routerbot.
pub struct TelegramAdapter {
    bot: Bot,
    state: Arc<RuntimeState>,
}

impl TelegramAdapter {
    /// Creates an adapter from a bot token and application services.
    #[must_use]
    pub fn new(
        token: impl Into<String>,
        command_service: CommandService,
        access_policy: TelegramAccessPolicy,
    ) -> Self {
        Self {
            bot: Bot::new(token),
            state: Arc::new(RuntimeState {
                command_service,
                access_policy,
                confirmations: Mutex::new(ConfirmationStore::default()),
            }),
        }
    }

    /// Runs the Telegram adapter using long polling until it is stopped.
    pub async fn run_long_polling(self) {
        let mut dispatcher = Dispatcher::builder(self.bot, handler_tree())
            .dependencies(teloxide::dptree::deps![self.state])
            .build();
        dispatcher.dispatch().await;
    }
}

struct RuntimeState {
    command_service: CommandService,
    access_policy: TelegramAccessPolicy,
    confirmations: Mutex<ConfirmationStore>,
}

fn handler_tree() -> UpdateHandler<HandlerError> {
    teloxide::dptree::entry()
        .branch(Update::filter_message().endpoint(handle_message))
        .branch(Update::filter_callback_query().endpoint(handle_callback))
}

async fn handle_message(
    bot: Bot,
    me: Me,
    message: Message,
    state: Arc<RuntimeState>,
) -> HandlerResult {
    let Some(text) = message.text() else {
        return Ok(());
    };
    let command = match TelegramCommand::parse(text, Some(me.username())) {
        Ok(Some(command)) => command,
        Ok(None) => return Ok(()),
        Err(error) => {
            bot.send_message(message.chat.id, error.to_string()).await?;
            return Ok(());
        }
    };
    let user_id = message.from.as_ref().map(|user| user.id.0);
    if !state.access_policy.is_allowed(message.chat.id.0, user_id) {
        bot.send_message(message.chat.id, ACCESS_DENIED).await?;
        return Ok(());
    }
    let command = match command.into_bot_command() {
        Ok(command) => command,
        Err(error) => {
            bot.send_message(message.chat.id, error.to_string()).await?;
            return Ok(());
        }
    };

    if command.action_risk() == ActionRisk::Dangerous {
        if let Err(error) = state.command_service.authorize(&command) {
            send_app_error(&bot, message.chat.id, &error).await?;
            return Ok(());
        }
        let Some(user_id) = user_id else {
            bot.send_message(message.chat.id, ACCESS_DENIED).await?;
            return Ok(());
        };
        let keyboard = InlineKeyboardMarkup::new([[
            InlineKeyboardButton::callback("Confirm", CONFIRM_CALLBACK),
            InlineKeyboardButton::callback("Cancel", CANCEL_CALLBACK),
        ]]);
        let confirmation_message = bot
            .send_message(message.chat.id, format_confirmation(&command))
            .reply_markup(keyboard)
            .await?;
        state.confirmations.lock().expect("confirmation store lock must not be poisoned").insert(
            ConfirmationKey {
                chat_id: confirmation_message.chat.id.0,
                message_id: confirmation_message.id.0,
            },
            user_id,
            command,
            Instant::now(),
        );
        return Ok(());
    }

    execute_and_reply(&bot, message.chat.id, command, &state.command_service).await
}

async fn handle_callback(
    bot: Bot,
    query: CallbackQuery,
    state: Arc<RuntimeState>,
) -> HandlerResult {
    let Some(message) = query.message.as_ref() else {
        answer_callback(&bot, query, "Confirmation is no longer available.", true).await?;
        return Ok(());
    };
    let chat_id = message.chat().id;
    let message_id = message.id();
    let user_id = query.from.id.0;
    if !state.access_policy.is_allowed(chat_id.0, Some(user_id)) {
        answer_callback(&bot, query, ACCESS_DENIED, true).await?;
        return Ok(());
    }

    let confirm = match query.data.as_deref() {
        Some(CONFIRM_CALLBACK) => true,
        Some(CANCEL_CALLBACK) => false,
        _ => {
            answer_callback(&bot, query, "Invalid confirmation.", true).await?;
            return Ok(());
        }
    };
    let resolution =
        state.confirmations.lock().expect("confirmation store lock must not be poisoned").resolve(
            ConfirmationKey { chat_id: chat_id.0, message_id: message_id.0 },
            user_id,
            confirm,
            Instant::now(),
        );

    match resolution {
        ConfirmationResolution::Confirmed(command) => {
            answer_callback(&bot, query, "Confirmed.", false).await?;
            clear_keyboard(&bot, chat_id, message_id).await?;
            let text = match state.command_service.execute(command).await {
                Ok(response) => format_command_response(&response),
                Err(error) => {
                    log_app_error(&error);
                    format_app_error(&error).to_owned()
                }
            };
            bot.edit_message_text(chat_id, message_id, text).await?;
        }
        ConfirmationResolution::Canceled => {
            answer_callback(&bot, query, "Canceled.", false).await?;
            clear_keyboard(&bot, chat_id, message_id).await?;
            bot.edit_message_text(chat_id, message_id, "Action canceled.").await?;
        }
        ConfirmationResolution::Expired => {
            answer_callback(&bot, query, "Confirmation expired.", true).await?;
            clear_keyboard(&bot, chat_id, message_id).await?;
            bot.edit_message_text(chat_id, message_id, "Confirmation expired.").await?;
        }
        ConfirmationResolution::Forbidden => {
            answer_callback(&bot, query, ACCESS_DENIED, true).await?;
        }
        ConfirmationResolution::Missing => {
            answer_callback(&bot, query, "Confirmation is no longer available.", true).await?;
            clear_keyboard(&bot, chat_id, message_id).await?;
        }
    }
    Ok(())
}

async fn execute_and_reply(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    command: BotCommand,
    command_service: &CommandService,
) -> HandlerResult {
    let text = match command_service.execute(command).await {
        Ok(response) => format_command_response(&response),
        Err(error) => {
            log_app_error(&error);
            format_app_error(&error).to_owned()
        }
    };
    bot.send_message(chat_id, text).await?;
    Ok(())
}

async fn send_app_error(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    error: &AppError,
) -> HandlerResult {
    log_app_error(error);
    bot.send_message(chat_id, format_app_error(error)).await?;
    Ok(())
}

fn log_app_error(error: &AppError) {
    if !matches!(error, AppError::Unauthorized { .. }) {
        log::error!("Telegram command failed: {error}");
    }
}

async fn answer_callback(
    bot: &Bot,
    query: CallbackQuery,
    text: &'static str,
    show_alert: bool,
) -> HandlerResult {
    bot.answer_callback_query(query.id).text(text).show_alert(show_alert).await?;
    Ok(())
}

async fn clear_keyboard(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    message_id: MessageId,
) -> HandlerResult {
    bot.edit_message_reply_markup(chat_id, message_id)
        .reply_markup(InlineKeyboardMarkup::default())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatcher_schema_can_be_constructed() {
        let _handler = handler_tree();
    }
}
