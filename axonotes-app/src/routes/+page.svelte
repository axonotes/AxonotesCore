<script lang="ts">
  import { RefreshCcw } from "@lucide/svelte";
  import {onMount} from "svelte";
  import {app} from "$lib/stores/app";

  onMount(async () => {
    await app.initialize();
  });
</script>

<div class="grid h-full w-full items-center">
  <div class="m-auto">
    <RefreshCcw class="animate-spin" />
  </div>
</div>
