<script lang="ts">
    import {CheckCircle} from "@lucide/svelte";
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-success-500 dark:shadow-success-900 shadow-success-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col items-center space-y-6 text-center">
            <CheckCircle class="text-success-500 h-16 w-16" />

            <div>
                <h1 class="h2 mb-2">Authorization Successful!</h1>
                <p class="text-surface-600 dark:text-surface-400">
                    Your desktop application has been authorized successfully.
                </p>
            </div>

            <div class="card preset-filled-success-100-900 rounded-lg p-4">
                <p class="text-sm">
                    You can now close this window and return to your desktop
                    application.
                </p>
            </div>
        </div>
    </div>
</div>
