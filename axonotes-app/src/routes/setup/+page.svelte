<script lang="ts">
  import {onMount} from "svelte";
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {setupStore} from "$lib/stores/setup";

  onMount(() => {
    // Reset setup state and redirect to step 1
    setupStore.reset();
    goto(resolve("/setup/master-password"), {replaceState: true});
  });
</script>

<div class="bg-background flex min-h-full w-full items-center justify-center">
  <div
    class="border-primary h-8 w-8 animate-spin rounded-full border-4 border-t-transparent"
  ></div>
</div>
