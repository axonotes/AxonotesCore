use std::collections::HashMap;
use log::error;
use crate::auth::jwt::decode_jwt_unsafe;
use crate::module_bindings;

pub struct Account {
    id: String,
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

pub struct AccountManager {
    accounts: HashMap<String, Account>,
    active_account: Option<String>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            active_account: None,
        }
    }

    pub fn add_account(&mut self, access_token: String, refresh_token: String, expires_at: u64) {
        let claims = match decode_jwt_unsafe(access_token.as_str()) {
            Ok(claims) => claims,
            Err(_) => {
                error!("Invalid JWT");
                return;
            }
        };

        let id = format!("{}-{}", claims.sub, claims.iss);

        // check if account already exists
        if self.accounts.contains_key(&id) {
            error!("Account with ID {} already exists", id);
            return;
        }

        let account = Account {
            id,
            access_token,
            refresh_token,
            expires_at,
        };
        self.accounts.insert(account.id.clone(), account);
    }

    pub fn set_active_account(&mut self, id: &str) -> Result<(), String> {
        if self.accounts.contains_key(id) {
            self.active_account = Some(id.to_string());
            Ok(())
        } else {
            Err(format!("Account with ID {} does not exist", id))
        }
    }

    pub fn get_active_account(&self) -> Option<&Account> {
        self.active_account.as_ref().and_then(|id| self.accounts.get(id))
    }

    pub fn remove_account(&mut self, id: &str) -> Result<(), String> {
        if self.accounts.remove(id).is_some() {
            if self.active_account.as_deref() == Some(id) {
                self.active_account = None;
            }
            Ok(())
        } else {
            Err(format!("Account with ID {} does not exist", id))
        }
    }
}