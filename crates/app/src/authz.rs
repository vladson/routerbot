//! Authorization service.

use std::collections::HashSet;

use routerbot_core::{BotCommand, Permission};

use crate::AppError;

/// Deny-by-default permission service.
#[derive(Debug, Clone, Default)]
pub struct AuthzService {
    allowed_permissions: HashSet<Permission>,
}

impl AuthzService {
    /// Creates a service that denies every command.
    #[must_use]
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// Creates a service that allows the provided permissions.
    #[must_use]
    pub fn allow_permissions(permissions: impl IntoIterator<Item = Permission>) -> Self {
        Self { allowed_permissions: permissions.into_iter().collect() }
    }

    /// Returns whether the command is allowed.
    #[must_use]
    pub fn is_allowed(&self, command: &BotCommand) -> bool {
        self.allowed_permissions.contains(&command.required_permission())
    }

    /// Authorizes a command.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Unauthorized`] when the required command permission
    /// is not allowed.
    pub fn authorize(&self, command: &BotCommand) -> Result<(), AppError> {
        let permission = command.required_permission();
        if self.allowed_permissions.contains(&permission) {
            Ok(())
        } else {
            Err(AppError::Unauthorized { permission })
        }
    }
}
