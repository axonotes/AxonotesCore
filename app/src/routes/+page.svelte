<script lang="ts">
    import { authManager } from '$lib/auth.svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';

    interface ConfigInfo {
        config: {
            dashboard_url: string;
            spacetimedb_url: string;
            spacetimedb_database: string;
            log_level: string;
        };
        config_path: string;
    }

    let configInfo = $state<ConfigInfo | null>(null);
    let showConfig = $state(false);

    onMount(async () => {
        try {
            configInfo = await invoke<ConfigInfo>('get_config_info');
        } catch (error) {
            console.error('Failed to load config:', error);
        }
    });

    function handleLogin() {
        authManager.initiateAuth(false);
    }

    function handleForceLogin() {
        authManager.initiateAuth(true);
    }

    function handleLogout() {
        authManager.logout();
    }

    function toggleConfig() {
        showConfig = !showConfig;
    }

    function handleRefresh() {
        authManager.refreshToken()
    }
</script>

<div class="container">
    <header>
        <h1>Axonotes Desktop</h1>
        <p>Phase 2: Authentication Testing</p>
    </header>

    <main>
        <div class="auth-section">
            <h2>Authentication Status</h2>

            {#if authManager.state.isWaitingForAuth}
                <div class="status waiting">
                    <div class="spinner"></div>
                    <p>Waiting for authentication...</p>
                    <p class="hint">Complete the login process in your browser</p>
                </div>
            {:else if authManager.state.isAuthenticated}
                <div class="status authenticated">
                    <h3>✅ Authenticated</h3>
                    <div class="user-info">
                        <p><strong>Email:</strong> {authManager.state.userInfo?.email}</p>
                        <p><strong>User ID:</strong> {authManager.state.userInfo?.id}</p>
                    </div>
                    <button onclick={handleLogout} class="btn btn-logout">Logout</button>
                </div>
            {:else}
                <div class="status unauthenticated">
                    <h3>🔐 Not Authenticated</h3>
                    <p>Please log in to continue</p>
                    <div class="auth-buttons">
                        <button onclick={handleLogin} class="btn btn-primary">Login</button>
                        <button onclick={handleForceLogin} class="btn btn-secondary">Force New Login</button>
                    </div>
                </div>
            {/if}

            {#if authManager.error}
                <div class="error">
                    <h4>Error:</h4>
                    <p>{authManager.error}</p>
                </div>
            {/if}
        </div>

        <div class="config-section">
            <h2>Configuration</h2>
            <button onclick={toggleConfig} class="btn btn-outline">
                {showConfig ? 'Hide' : 'Show'} Config
            </button>

            {#if showConfig && configInfo}
                <div class="config-details">
                    <h3>Current Configuration</h3>
                    <div class="config-item">
                        <strong>Config Path:</strong>
                        <code>{configInfo.config_path}</code>
                    </div>
                    <div class="config-item">
                        <strong>Dashboard URL:</strong>
                        <code>{configInfo.config.dashboard_url}</code>
                    </div>
                    <div class="config-item">
                        <strong>SpacetimeDB URL:</strong>
                        <code>{configInfo.config.spacetimedb_url}</code>
                    </div>
                    <div class="config-item">
                        <strong>Database:</strong>
                        <code>{configInfo.config.spacetimedb_database}</code>
                    </div>
                    <div class="config-item">
                        <strong>Log Level:</strong>
                        <code>{configInfo.config.log_level}</code>
                    </div>
                </div>
            {/if}
        </div>
    </main>
</div>

<style>
    .container {
        max-width: 800px;
        margin: 0 auto;
        padding: 2rem;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    }

    header {
        text-align: center;
        margin-bottom: 2rem;
    }

    header h1 {
        color: #2563eb;
        margin: 0;
    }

    header p {
        color: #6b7280;
        margin: 0.5rem 0 0 0;
    }

    .auth-section, .config-section {
        background: #f8fafc;
        border-radius: 8px;
        padding: 1.5rem;
        margin-bottom: 2rem;
    }

    .auth-section h2, .config-section h2 {
        margin-top: 0;
        color: #374151;
    }

    .status {
        padding: 1rem;
        border-radius: 6px;
        margin-bottom: 1rem;
    }

    .status.waiting {
        background: #fef3c7;
        border: 1px solid #f59e0b;
    }

    .status.authenticated {
        background: #d1fae5;
        border: 1px solid #10b981;
    }

    .status.unauthenticated {
        background: #fee2e2;
        border: 1px solid #ef4444;
    }

    .spinner {
        border: 2px solid #f3f4f6;
        border-top: 2px solid #3b82f6;
        border-radius: 50%;
        width: 20px;
        height: 20px;
        animation: spin 1s linear infinite;
        margin: 0 auto 1rem auto;
    }

    @keyframes spin {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
    }

    .user-info {
        background: white;
        padding: 1rem;
        border-radius: 4px;
        margin: 1rem 0;
    }

    .user-info p {
        margin: 0.5rem 0;
    }

    .auth-buttons {
        display: flex;
        gap: 1rem;
        margin-top: 1rem;
    }

    .btn {
        padding: 0.5rem 1rem;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 1rem;
        transition: all 0.2s;
    }

    .btn-primary {
        background: #3b82f6;
        color: white;
    }

    .btn-primary:hover {
        background: #2563eb;
    }

    .btn-secondary {
        background: #6b7280;
        color: white;
    }

    .btn-secondary:hover {
        background: #4b5563;
    }

    .btn-logout {
        background: #ef4444;
        color: white;
    }

    .btn-logout:hover {
        background: #dc2626;
    }

    .btn-outline {
        background: transparent;
        border: 1px solid #d1d5db;
        color: #374151;
    }

    .btn-outline:hover {
        background: #f9fafb;
    }

    .error {
        background: #fef2f2;
        border: 1px solid #f87171;
        border-radius: 4px;
        padding: 1rem;
        margin-top: 1rem;
    }

    .error h4 {
        color: #dc2626;
        margin: 0 0 0.5rem 0;
    }

    .error p {
        color: #991b1b;
        margin: 0;
    }

    .config-details {
        margin-top: 1rem;
        background: white;
        padding: 1rem;
        border-radius: 4px;
    }

    .config-item {
        margin-bottom: 0.5rem;
    }

    .config-item code {
        background: #f3f4f6;
        padding: 0.25rem 0.5rem;
        border-radius: 3px;
        font-size: 0.875rem;
    }

    .hint {
        font-size: 0.875rem;
        color: #6b7280;
        margin-top: 0.5rem;
    }
</style>