<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {isReady, isInitialized} from "$lib/stores/app";
  import {Loader2} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  // Redirect to /app when ready
  $effect(() => {
    if ($isInitialized && $isReady) {
      goto(resolve("/app"), {replaceState: true});
    }
  });
</script>

<!--
  Root page - loading/redirect page only.
  Redirects to /app when authenticated, layout handles auth redirects.
-->

<div class="flex min-h-[50vh] items-center justify-center">
  <div class="text-center">
    <Loader2 class="text-muted-foreground mx-auto h-8 w-8 animate-spin" />
    <p class="text-muted-foreground mt-4 text-sm">{m.common_loading()}</p>
  </div>
</div>
