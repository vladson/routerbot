//! In-memory dangerous-action confirmations.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use routerbot_core::BotCommand;

pub(crate) const CONFIRMATION_TTL: Duration = Duration::from_mins(2);
const MAX_PENDING_CONFIRMATIONS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ConfirmationKey {
    pub(crate) chat_id: i64,
    pub(crate) message_id: i32,
}

#[derive(Debug, Clone)]
struct PendingConfirmation {
    user_id: u64,
    command: BotCommand,
    created_at: Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConfirmationResolution {
    Confirmed(BotCommand),
    Canceled,
    Expired,
    Forbidden,
    Missing,
}

#[derive(Debug, Default)]
pub(crate) struct ConfirmationStore {
    pending: HashMap<ConfirmationKey, PendingConfirmation>,
}

impl ConfirmationStore {
    pub(crate) fn insert(
        &mut self,
        key: ConfirmationKey,
        user_id: u64,
        command: BotCommand,
        now: Instant,
    ) {
        self.prune_expired(now);
        if self.pending.len() >= MAX_PENDING_CONFIRMATIONS {
            self.evict_oldest();
        }
        self.pending.insert(key, PendingConfirmation { user_id, command, created_at: now });
    }

    pub(crate) fn resolve(
        &mut self,
        key: ConfirmationKey,
        user_id: u64,
        confirm: bool,
        now: Instant,
    ) -> ConfirmationResolution {
        let Some(pending) = self.pending.get(&key) else {
            return ConfirmationResolution::Missing;
        };
        if now.duration_since(pending.created_at) >= CONFIRMATION_TTL {
            self.pending.remove(&key);
            return ConfirmationResolution::Expired;
        }
        if pending.user_id != user_id {
            return ConfirmationResolution::Forbidden;
        }

        let pending = self.pending.remove(&key).expect("pending confirmation must exist");
        if confirm {
            ConfirmationResolution::Confirmed(pending.command)
        } else {
            ConfirmationResolution::Canceled
        }
    }

    fn prune_expired(&mut self, now: Instant) {
        self.pending.retain(|_, pending| now.duration_since(pending.created_at) < CONFIRMATION_TTL);
    }

    fn evict_oldest(&mut self) {
        if let Some(key) =
            self.pending.iter().min_by_key(|(_, pending)| pending.created_at).map(|(key, _)| *key)
        {
            self.pending.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(message_id: i32) -> ConfirmationKey {
        ConfirmationKey { chat_id: 1, message_id }
    }

    #[test]
    fn confirmation_is_single_use() {
        let now = Instant::now();
        let mut store = ConfirmationStore::default();
        store.insert(key(1), 7, BotCommand::RebootDevice, now);

        assert_eq!(
            store.resolve(key(1), 7, true, now),
            ConfirmationResolution::Confirmed(BotCommand::RebootDevice)
        );
        assert_eq!(store.resolve(key(1), 7, true, now), ConfirmationResolution::Missing);
    }

    #[test]
    fn foreign_user_does_not_consume_confirmation() {
        let now = Instant::now();
        let mut store = ConfirmationStore::default();
        store.insert(key(1), 7, BotCommand::RebootDevice, now);

        assert_eq!(store.resolve(key(1), 8, true, now), ConfirmationResolution::Forbidden);
        assert_eq!(store.resolve(key(1), 7, false, now), ConfirmationResolution::Canceled);
    }

    #[test]
    fn expired_confirmation_is_removed() {
        let now = Instant::now();
        let mut store = ConfirmationStore::default();
        store.insert(key(1), 7, BotCommand::RebootDevice, now);

        assert_eq!(
            store.resolve(key(1), 7, true, now + CONFIRMATION_TTL),
            ConfirmationResolution::Expired
        );
        assert_eq!(store.resolve(key(1), 7, true, now), ConfirmationResolution::Missing);
    }

    #[test]
    fn capacity_evicts_oldest_confirmation() {
        let now = Instant::now();
        let mut store = ConfirmationStore::default();
        for message_id in 0..MAX_PENDING_CONFIRMATIONS {
            store.insert(
                key(i32::try_from(message_id).unwrap()),
                7,
                BotCommand::RebootDevice,
                now + Duration::from_millis(u64::try_from(message_id).unwrap()),
            );
        }
        store.insert(key(999), 7, BotCommand::RebootDevice, now + Duration::from_secs(1));

        assert_eq!(store.resolve(key(0), 7, true, now), ConfirmationResolution::Missing);
        assert_eq!(store.pending.len(), MAX_PENDING_CONFIRMATIONS);
    }
}
