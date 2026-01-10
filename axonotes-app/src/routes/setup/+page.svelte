<script lang="ts">
  import {onMount} from "svelte";
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {setupStore} from "$lib/stores/setup";
  import {Spinner} from "$lib/components/ui/spinner";

  onMount(() => {
    // Reset setup state and redirect to step 1
    setupStore.reset();
    goto(resolve("/setup/master-password"), {replaceState: true});
  });
</script>

<div class="bg-background flex min-h-full w-full items-center justify-center">
  <Spinner size="lg" />
</div>
