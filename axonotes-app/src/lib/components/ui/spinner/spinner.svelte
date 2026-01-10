<script lang="ts">
  import {Loader2} from "@lucide/svelte";
  import {cn} from "$lib/utils.js";

  interface Props {
    class?: string;
    size?: "sm" | "md" | "lg";
  }

  let {class: className, size = "md"}: Props = $props();

  const sizeClasses = {
    sm: "h-4 w-4",
    md: "h-6 w-6",
    lg: "h-8 w-8",
  };
</script>

<Loader2
  class={cn("text-muted-foreground animate-spin", sizeClasses[size], className)}
/>
