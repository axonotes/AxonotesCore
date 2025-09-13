use crate::user_profile::auth::manager::{
    AuthorizationCompleteResponse, StartNewAuthFlowResponse,
};
use crate::user_profile::UserProfile;
use anyhow::Result;
use serde::{Serialize};
use tauri::State;
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct AddAndAuthNewUserResponse {
    start_new_auth_flow_response: StartNewAuthFlowResponse,
    user_profile_id: u16,
}

pub struct AppState {
    user_profiles: Mutex<Vec<UserProfile>>,
}

impl Default for AppState {
    /// Creates a default AppState with an empty, mutex-protected list of user profiles.
    ///
    /// # Examples
    ///
    /// ```
    /// let state = app_state::AppState::default();
    /// let profiles = state.user_profiles.blocking_lock();
    /// assert_eq!(profiles.len(), 0);
    /// ```
    fn default() -> Self {
        Self {
            user_profiles: Mutex::new(Vec::new()),
        }
    }
}

/// Creates a new in-memory user profile, initiates its authentication flow, stores the profile in the shared AppState, and returns the authentication start response along with the new profile's ID.
///
/// The function always appends the newly created profile into `AppState.user_profiles` before returning. The returned `AddAndAuthNewUserResponse` contains the value produced by `UserProfile::start_new_auth_flow().await` and the `user_profile_id` assigned to the new profile.
///
/// # Examples
///
/// ```no_run
/// # async fn example(state: tauri::State<'_, app_state::AppState>) {
/// let res = app_state::add_and_auth_new_user(state).await.unwrap();
/// // res.start_new_auth_flow_response can be used to continue the auth flow (e.g., open a browser)
/// let new_id = res.user_profile_id;
/// println!("Created profile id: {}", new_id);
/// # }
/// ```
#[tauri::command]
pub async fn add_and_auth_new_user(
    state: State<'_, AppState>,
) -> Result<AddAndAuthNewUserResponse, String> {
    let mut new_user_profile = UserProfile::new();
    let res = new_user_profile.start_new_auth_flow().await;
    let id = new_user_profile.user_profile_id;

    let mut profiles = state.user_profiles.lock().await;
    profiles.push(new_user_profile);

    Ok(AddAndAuthNewUserResponse {
        start_new_auth_flow_response: res,
        user_profile_id: id,
    })
}

/// Waits for an existing user profile to complete its authorization flow.
///
/// Looks up the profile by `user_profile_id` in the shared `AppState`. If found,
/// this function awaits the profile's authorization completion and returns the
/// resulting `AuthorizationCompleteResponse`. If no profile matches the given
/// id, it returns `Err("User profile not found")`.
///
/// # Examples
///
/// ```no_run
/// # use tauri::State;
/// # async fn example(state: State<'_, crate::app_state::AppState>) {
/// let result = crate::app_state::wait_for_user_auth(state, 1).await;
/// match result {
///     Ok(auth_complete) => { /* use `auth_complete` */ }
///     Err(e) => eprintln!("error: {}", e),
/// }
/// # }
/// ```
#[tauri::command(rename_all = "snake_case")]
pub async fn wait_for_user_auth(
    state: State<'_, AppState>,
    user_profile_id: u16,
) -> Result<AuthorizationCompleteResponse, String> {
    let mut profiles = state.user_profiles.lock().await;

    match profiles
        .iter_mut()
        .find(|profile| profile.user_profile_id == user_profile_id)
    {
        Some(profile) => Ok(profile.wait_for_authorization().await),
        None => Err("User profile not found".into()),
    }
}

/// Refreshes the stored OAuth tokens for the given user profile.
///
/// Returns `Ok(true)` if the profile was found and tokens were refreshed successfully,
/// `Ok(false)` if the profile was found but the refresh operation failed, or
/// `Err(String)` if no profile with the provided `user_profile_id` exists.
///
/// # Examples
///
/// ```no_run
/// // This function is intended to be invoked via Tauri; shown here for illustration:
/// let result = refresh_user_token(state, 42).await;
/// match result {
///     Ok(true) => println!("Tokens refreshed"),
///     Ok(false) => println!("Refresh attempted but failed"),
///     Err(e) => println!("Profile lookup failed: {}", e),
/// }
/// ```
#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_user_token(
    state: State<'_, AppState>,
    user_profile_id: u16,
) -> Result<bool, String> {
    let mut profiles = state.user_profiles.lock().await;

    match profiles
        .iter_mut()
        .find(|profile| profile.user_profile_id == user_profile_id)
    {
        Some(profile) => Ok(profile.refresh_tokens().await),
        None => Err("User profile not found".into())
    }
}

/// Remove a user profile from the in-memory application state by its ID.
///
/// On success this removes all profiles with the given `user_profile_id` from
/// the shared `AppState` and returns `Ok(true)`. If no profile with that ID
/// exists, returns `Err("User profile not found")`.
///
/// # Examples
///
/// ```no_run
/// // Given a Tauri `State<AppState>` named `state` and a profile ID:
/// // let state: State<'_, AppState> = ...;
/// // let removed = remove_user(state, 42).await?;
/// // assert!(removed);
/// ```
#[tauri::command(rename_all = "snake_case")]
pub async fn remove_user(
    state: State<'_, AppState>,
    user_profile_id: u16,
) -> Result<bool, String> {
    let mut profiles = state.user_profiles.lock().await;

    let initial_len = profiles.len();
    profiles.retain(|profile| profile.user_profile_id != user_profile_id);

    if profiles.len() < initial_len {
        Ok(true)
    } else {
        Err("User profile not found".into())
    }
}