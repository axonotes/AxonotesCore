<script lang="ts">
  import {open} from "@tauri-apps/plugin-shell";
  import {Button} from "$lib/components/ui/button";
  import {ExternalLink, Github} from "@lucide/svelte";
  import Discord from "$lib/components/icons/Discord.svelte";
  import * as m from "$lib/paraglide/messages.js";

  function openLink(url: string) {
    open(url);
  }
</script>

<div class="flex h-full w-full justify-center overflow-auto p-8">
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-2">
      <h1 class="text-2xl leading-tight font-semibold tracking-tight">
        {m.settings_about_title()}
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        Version <code
          class="bg-muted text-foreground rounded px-1.5 py-0.5 font-mono text-xs"
          >Beta-1</code
        >
      </p>
    </div>

    <!-- Links -->
    <section class="space-y-3">
      <Button
        variant="outline"
        class="w-full justify-start"
        onclick={() =>
          openLink("https://github.com/axonotes/AxonotesCore/tree/beta-1")}
      >
        <Github class="mr-3 h-4 w-4" />
        GitHub
        <ExternalLink class="text-muted-foreground ml-auto h-3 w-3" />
      </Button>

      <Button
        variant="outline"
        class="w-full justify-start"
        onclick={() => openLink("https://discord.gg/myBMaaDeQu")}
      >
        <Discord class="mr-3 h-4 w-4" />
        Discord
        <ExternalLink class="text-muted-foreground ml-auto h-3 w-3" />
      </Button>
    </section>
  </div>
</div>
