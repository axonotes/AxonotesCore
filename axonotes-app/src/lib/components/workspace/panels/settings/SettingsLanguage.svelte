<script lang="ts">
  import {Input} from "$lib/components/ui/input";
  import {Globe} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";
</script>

<div class="flex h-full w-full justify-center overflow-auto p-8">
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-2">
      <h1 class="text-2xl leading-tight font-semibold tracking-tight">
        {m.settings_language_title()}
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        {m.settings_language_description()}
      </p>
    </div>

    <!-- Language Selection -->
    <section>
      <div class="space-y-4 rounded-lg border p-4">
        <div class="relative">
          <Globe
            class="text-muted-foreground absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2"
          />
          <Input value="English" disabled class="pl-10" />
        </div>
        <p class="text-muted-foreground text-xs">
          {m.settings_language_coming_soon()}
        </p>
      </div>
    </section>
  </div>
</div>
