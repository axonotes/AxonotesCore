# Auth Server

This document describes the setup, configuration, and authentication flow for the auth server. It enables client applications to obtain JWTs for authenticated users via a secure PKCE-like flow involving client-side Ed25519 signatures.

**Note on Paths:** The term `/auth` used throughout this document refers to the root directory of this project (i.e., the directory containing this `README.md` file). All commands should be executed from this directory.

## Installation

### Prerequisites

*   [Bun](https://bun.sh/) installed on your system.

### 1. Install Dependencies

Navigate to the `/auth` directory and run the following command to install the necessary project dependencies:

```bash
bun install
```

## Configuration

### 1. Generate Secrets

To generate essential cryptographic keys (e.g., for signing tokens or other internal purposes), run the `generate-keys` script. This script will output the generated keys into a `.env.generated` file in the `/auth` directory.

```bash
bun run generate-keys
```

### 2. Setup Environment File

The server requires an `.env` file for its configuration.

1.  **Create `.env` file:** Copy the example environment file to create your local configuration:
    ```bash
    cp .example.env .env
    ```
2.  **Populate with generated secrets:** Open the newly created `.env` file. Copy the variable values from the `.env.generated` file (created in the previous step) into your `.env` file.
3.  **Configure remaining variables:** Review `.example.env` and ensure all other necessary environment variables in your `.env` file are correctly set for your specific deployment and identity providers (e.g., API keys, `CLIENT_ED25519_PUBLIC_KEY_B64URL`).

## Running the Server

### Run in Development Mode

To start the development server, run the following command in the `/auth` directory:

```bash
bun run dev
```

The server should now be running, usually on `http://localhost:5173`.

## Authentication Flow

This section details the process by which a client application can obtain a JWT from this auth server.

### Prerequisites for the Client Application

1.  **Ed25519 Key Pair:**
   *   The client application must possess an Ed25519 private key.
   *   The corresponding public key, Base64URL encoded (referred to as `CLIENT_ED25519_PUBLIC_KEY_B64URL`), must be configured as an environment variable on this auth server.
2.  **Cryptography Capabilities:** The client application needs to be able to:
   *   Generate cryptographically strong random strings (for `code_verifier` and `request_id`).
   *   Perform SHA256 hashing.
   *   Base64URL encode and decode data.
   *   Generate Ed25519 signatures using its private key.

### Client-Side JWT Acquisition Flow

Here's the step-by-step process:

**Step 1: Generate PKCE Parameters & Request ID (Client-Side)**

*   **`code_verifier`**: The client generates a high-entropy cryptographic random string. This is the `code_verifier`.
   *   *Example*: `_aB3cD4eF5gH6iJ7kL8mN9oP0qR1sT2uV3wX4yZ5-A6bC7dE8fG9hI0jK1lM2nO3pQ4rS5tU6vW7xY8z`
*   **`code_challenge`**: The client calculates the SHA256 hash of the `code_verifier` and then Base64URL encodes the binary hash. This is the `code_challenge`.
   *   *Process*: code_challenge = Base64URL(SHA256(code_verifier))
*   **`request_id`**: The client generates a unique identifier for this authentication attempt (e.g., a UUIDv4). This `request_id` must be unpredictable.
   *   *Example*: `abc123xyz789-def456-ghi789`

**Step 2: Generate Client Signature (Client-Side)**

*   The client takes the `code_challenge` string (from Step 1).
*   It signs this `code_challenge` string using its Ed25519 private key.
*   The resulting binary signature is then Base64URL encoded. This is the `client_signature` (parameter name `cs`).
   *   *Process*: client_signature = Base64URL(Ed25519Sign(private_key, UTF8Encode(code_challenge)))

**Step 3: Construct and Open Initiation URL (Client-Side & Server-Side)**

*   **Client Action:**
   *   The client constructs the initiation URL with the generated parameters:  
      `rid`: `request_id`  
      `ch`: `code_challenge`  
      `cs`: `client_signature`  
   *   *URL*: `/auth/initiate?rid=<request_id>&ch=<code_challenge>&cs=<client_signature>`
   *   The client opens this URL in the user's web browser (e.g., via `window.open()` for web apps, or by instructing the user for CLI/desktop apps).

*   **Server-Side Handling (`/auth/initiate` endpoint):**
   *   The server receives the request.
   *   It extracts `rid` (request ID), `ch` (code challenge), and `cs` (client signature).
   *   It verifies the `client_signature` (`cs`) against the received `code_challenge` (`ch`) using the pre-configured `CLIENT_ED25519_PUBLIC_KEY_B64URL` associated with the client.
   *   If the signature is valid:  
      *   It stores `{ type: 'PENDING', codeChallenge: ch }` in a temporary server-side cache (e.g., `jwtCache`) keyed by the `request_id`. This entry should have a short expiry time.  
      *   It sets an HTTP-only cookie (e.g., `app.request-id`) with the value of `request_id`.  
      *   It redirects the browser to the main sign-in page (e.g., `/signin`).  
   *   If signature verification fails, it returns an error page.

**Step 4: User Authentication in Browser (User Interaction & Server-Side)**

*   **User Interaction:**
   *   The browser is redirected to the `/signin` page.
   *   The user interacts with the sign-in page (managed by the auth server, using Auth.js) and authenticates using their chosen identity provider (e.g., Google, GitHub).

*   **Server-Side Handling (`/auth/complete-link` callback):**
   *   Upon successful authentication with the identity provider, the provider redirects back to the auth server.
   *   The server (at `/auth/complete-link` endpoint):  
      *   Confirms an active user session (`event.locals.auth()`).  
      *   Retrieves the `request_id` from the `app.request-id` cookie.  
      *   If a valid session and `request_id` exist:  
         *   Generates the final JWT (`spacetimeToken`) for the authenticated user.
         *   Retrieves the `PENDING` state (including the original `codeChallenge`) from the cache using `request_id`.
         *   Updates the cache entry for `request_id` to `{ type: 'USER_AUTH_COMPLETED', codeChallenge: <original_code_challenge_from_cache>, jwtToken: <generated_jwt> }`.
         *   Deletes the `app.request-id` cookie.
         *   Redirects the browser to a success page (`/success`), informing the user they can return to the client application. The JWT is now securely stored server-side, associated with the `request_id`, awaiting exchange.

**Step 5: Client Polls for Authentication Status (Client-Side & Server-Side)**

*   **Client Action:**
   *   While the user is authenticating in the browser (Step 4), the original client application (which still has its `request_id` and `code_verifier`) starts polling the token status endpoint:  
      *   `GET /api/auth/token/<request_id>`
   *   The client makes periodic GET requests to this endpoint.

*   **Server-Side Handling (`GET /api/auth/token/[requestId]` endpoint):**
   *   The server looks up the `request_id` in its cache.
   *   If the cache entry is `{ type: 'PENDING', ... }`, the server responds with:
       ```json
       {
         "status": "pending_user_authentication",
         "message": "User authentication is pending. Please continue login process in your browser."
       }
       ```
   *   Once the user completes Step 4 and the server updates the cache to `{ type: 'USER_AUTH_COMPLETED', ... }`, subsequent polls will trigger the server to respond with:
       ```json
       {
         "status": "ready_for_token_exchange",
         "message": "User authentication complete. Client can now exchange code_verifier for token via POST."
       }
       ```
   *   If the `request_id` is not found or has expired, an appropriate error is returned.

**Step 6: Client Exchanges Code Verifier for JWT (Client-Side & Server-Side)**

*   **Client Action:**
   *   When the client receives the `ready_for_token_exchange` status from the polling endpoint, it knows the user has successfully authenticated.
   *   The client then makes a `POST` request to the same token endpoint:  
      *   `POST /api/auth/token/<request_id>`  
      *   With a JSON body containing the original `code_verifier`:
         ```json
         {
           "code_verifier": "<original_code_verifier_from_Step_1>"
         }
         ```

*   **Server-Side Handling (`POST /api/auth/token/[requestId]` endpoint):**
   *   The server receives the `request_id` (from URL) and `code_verifier` (from POST body).
   *   It retrieves the `{ type: 'USER_AUTH_COMPLETED', codeChallenge, jwtToken }` state from the cache using `request_id`.
   *   **PKCE Verification:** It hashes the received `code_verifier` (SHA256 then Base64URL) and compares it to the stored `codeChallenge`.
   *   If the `code_verifier`'s hash matches the stored `codeChallenge` (PKCE check passes) and the state is `USER_AUTH_COMPLETED`:  
      *   It deletes the `request_id` entry from the cache (to ensure one-time use).  
      *   It responds with the JWT:
         ```json
         {
           "status": "success",
           "token_type": "Bearer",
           "access_token": "<the_actual_jwtToken>"
         }
         ```
   *   If the PKCE check fails, the state is invalid, `request_id` is not found, or the entry has expired, it returns an appropriate error (e.g., 400 Bad Request, 403 Forbidden).

**Step 7: Client Receives and Uses JWT (Client-Side)**

*   The client application receives the JSON response containing the `access_token`.
*   It can now store this token securely (e.g., in memory for the session) and use it for subsequent authenticated requests to protected resources.

### Security Benefits of this Flow

This authentication flow provides several security advantages:

*   **Client Authentication:** Only a client possessing the correct Ed25519 private key (corresponding to a public key configured on the server) can initiate the authentication process.
*   **Protection Against Authorization Code Interception:** Even if the redirection flow in the browser (during user login) is somehow compromised, an attacker cannot obtain the final JWT without the original `code_verifier` (thanks to PKCE).
*   **Linking Initial Request to Token Exchange:** The `request_id` ensures that the initial signed request from the client is cryptographically linked to the user authentication session and the final token exchange, preventing cross-request attacks.
*   **Mitigation of CSRF:** The use of `request_id` and client signatures helps protect the initiation step.
*   **Short-Lived State:** Server-side cached states (`PENDING`, `USER_AUTH_COMPLETED`) are temporary and should have short expiry times, reducing the window for potential misuse.
