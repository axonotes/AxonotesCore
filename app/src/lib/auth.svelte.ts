import { invoke } from "@tauri-apps/api/core";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";

export interface AuthState {
  isAuthenticated: boolean;
  userInfo?: {
    id: string;
    email: string;
  };
  isWaitingForAuth: boolean;
}

export interface TokenResponse {
  access_token: string;
  refresh_token: string;
  expires_at: number;
  user: {
    id: string;
    email: string;
  };
  workos_session_id?: string;
}

class AuthManager {
  private _state = $state<AuthState>({
    isAuthenticated: false,
    userInfo: undefined,
    isWaitingForAuth: false,
  });

  private _error = $state<string | null>(null);

  constructor() {
    this.initializeAuth();
    this.setupDeepLinkListener();
  }

  get state() {
    return this._state;
  }

  get error() {
    return this._error;
  }

  private async initializeAuth() {
    try {
      this._error = null;

      const authState = await invoke<AuthState>("get_auth_state");
      this._state.isAuthenticated = authState.isAuthenticated;
      this._state.userInfo = authState.userInfo;
      this._state.isWaitingForAuth = authState.isWaitingForAuth;
    } catch (error) {
      this._error = `Failed to initialize auth: ${error}`;
      console.error("Auth initialization failed:", error);
    }
  }

  private async setupDeepLinkListener() {
    try {
      await onOpenUrl((urls) => {
        console.log("Deep link received:", urls);

        for (const url of urls) {
          this.handleDeepLink(url);
        }
      });
      console.log("Deep link listener setup successfully");
    } catch (error) {
      console.error("Failed to setup deep link listener:", error);
    }
  }

  private async handleDeepLink(url: string) {
    try {
      const parsedUrl = new URL(url);

      console.log(parsedUrl);

      if (
        parsedUrl.protocol === "axonotes:" &&
        parsedUrl.pathname === "/success"
      ) {
        const sessionId = parsedUrl.searchParams.get("session_id");

        if (sessionId) {
          console.log(
            "Auth success deep link received, session ID:",
            sessionId,
          );

          const tokenResponse = await invoke<TokenResponse>(
            "handle_auth_callback",
            { sessionId },
          );
          console.log("Auth callback successful:", tokenResponse);

          this._state.isAuthenticated = true;
          this._state.userInfo = tokenResponse.user;
          this._state.isWaitingForAuth = false;
          this._error = null;
        } else {
          console.error("No session ID in deep link");
          this._error = "Invalid authentication response";
          this._state.isWaitingForAuth = false;
        }
      }
    } catch (error) {
      console.error("Deep link handling failed:", error);
      this._error = `Authentication failed: ${error}`;
      this._state.isWaitingForAuth = false;
    }
  }

  async initiateAuth(forceNewLogin = false) {
    try {
      this._error = null;

      // Get the auth URL from backend
      const response = await invoke<{ auth_url: string }>(
        "get_initiate_auth_url",
        {
          request: { force_new_login: forceNewLogin },
        },
      );

      console.log("Got auth URL:", response.auth_url);

      // Open URL in browser
      await openUrl(response.auth_url);

      // Update state to show we're waiting
      this._state.isWaitingForAuth = true;
    } catch (error) {
      this._error = `Failed to initiate auth: ${error}`;
      console.error("Auth initiation failed:", error);
    }
  }

  async logout() {
    try {
      await invoke("logout");
      this._state.isAuthenticated = false;
      this._state.userInfo = undefined;
      this._state.isWaitingForAuth = false;
      this._error = null;
    } catch (error) {
      this._error = `Logout failed: ${error}`;
      console.error("Logout failed:", error);
    }
  }

  async refreshToken(refreshToken: string) {
    try {
      const response = await invoke<TokenResponse>("refresh_token", {
        refreshToken,
      });
      return response;
    } catch (error) {
      this._error = `Token refresh failed: ${error}`;
      console.error("Token refresh failed:", error);
      throw error;
    }
  }
}

export const authManager = new AuthManager();
