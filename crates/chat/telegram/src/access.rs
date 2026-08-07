//! Telegram caller access policy.

use std::collections::HashSet;

/// Allow-list policy for Telegram chats and users.
///
/// A caller is allowed when either its chat or its user is present in the
/// corresponding allow list. Empty lists deny every caller.
#[derive(Debug, Clone, Default)]
pub struct TelegramAccessPolicy {
    allowed_chats: HashSet<i64>,
    allowed_users: HashSet<u64>,
}

impl TelegramAccessPolicy {
    /// Creates an allow-list policy.
    #[must_use]
    pub fn new(
        allowed_chats: impl IntoIterator<Item = i64>,
        allowed_users: impl IntoIterator<Item = u64>,
    ) -> Self {
        Self {
            allowed_chats: allowed_chats.into_iter().collect(),
            allowed_users: allowed_users.into_iter().collect(),
        }
    }

    /// Returns whether a Telegram chat and sender are allowed.
    ///
    /// Updates without a sender are denied unless their chat is explicitly
    /// allowed.
    #[must_use]
    pub fn is_allowed(&self, chat_id: i64, user_id: Option<u64>) -> bool {
        self.allowed_chats.contains(&chat_id)
            || user_id.is_some_and(|user_id| self.allowed_users.contains(&user_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_chat_or_user() {
        let policy = TelegramAccessPolicy::new([-100], [42]);

        assert!(policy.is_allowed(-100, None));
        assert!(policy.is_allowed(1, Some(42)));
        assert!(!policy.is_allowed(1, Some(7)));
    }

    #[test]
    fn empty_policy_denies_everyone() {
        let policy = TelegramAccessPolicy::default();

        assert!(!policy.is_allowed(1, Some(42)));
        assert!(!policy.is_allowed(1, None));
    }
}
