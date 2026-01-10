<script lang="ts">
  import {onMount} from "svelte";
  import {
    StorageService,
    type QuotaInfo,
    type CacheInfo,
  } from "$lib/services/storage";
  import {Button} from "$lib/components/ui/button";
  import {Progress} from "$lib/components/ui/progress";
  import {
    HardDrive,
    Database,
    Trash2,
    Loader2,
    RefreshCw,
  } from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let quota = $state<QuotaInfo | null>(null);
  let cache = $state<CacheInfo | null>(null);
  let loading = $state(true);
  let clearing = $state(false);
  let clearSuccess = $state("");

  async function loadData() {
    loading = true;
    try {
      const isInitialized = await StorageService.isInitialized();
      if (isInitialized) {
        // Load separately so one failure doesn't block the other
        const [quotaResult, cacheResult] = await Promise.allSettled([
          StorageService.getQuota(),
          StorageService.getCacheInfo(),
        ]);

        if (quotaResult.status === "fulfilled") {
          quota = quotaResult.value;
        } else {
          console.error("Failed to load quota:", quotaResult.reason);
        }

        if (cacheResult.status === "fulfilled") {
          cache = cacheResult.value;
        } else {
          console.error("Failed to load cache info:", cacheResult.reason);
        }
      }
    } catch (err) {
      console.error("Failed to load storage info:", err);
    } finally {
      loading = false;
    }
  }

  async function handleClearCache() {
    clearing = true;
    clearSuccess = "";
    try {
      await StorageService.clearCache();
      clearSuccess = m.settings_storage_cache_cleared();
      cache = await StorageService.getCacheInfo();
    } catch (err) {
      console.error("Failed to clear cache:", err);
    } finally {
      clearing = false;
    }
  }

  let usagePercent = $derived(
    quota ? Math.round((quota.usedBytes / quota.totalBytes) * 100) : 0
  );

  onMount(() => {
    loadData();
  });
</script>

<div class="flex h-full w-full justify-center overflow-auto p-8">
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-2">
      <h1 class="text-2xl leading-tight font-semibold tracking-tight">
        {m.settings_storage_title()}
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        {m.settings_storage_description()}
      </p>
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-12">
        <Loader2 class="text-muted-foreground h-6 w-6 animate-spin" />
      </div>
    {:else}
      <!-- Storage Quota -->
      <section class="space-y-4">
        <div class="space-y-4 rounded-lg border p-4">
          <div class="flex items-center gap-3">
            <HardDrive class="text-muted-foreground h-5 w-5" />
            <p class="text-sm font-medium">
              {m.settings_storage_quota_title()}
            </p>
          </div>

          {#if quota}
            <div class="space-y-2">
              <div class="flex justify-between text-xs">
                <span class="text-muted-foreground">
                  {m.settings_storage_quota_used({
                    used: quota.usedFormatted,
                    total: quota.totalFormatted,
                  })}
                </span>
                <span class="font-medium">{usagePercent}%</span>
              </div>
              <Progress value={usagePercent} class="h-2" />
            </div>
          {:else}
            <p class="text-muted-foreground text-xs">
              {m.settings_storage_not_initialized()}
            </p>
          {/if}
        </div>
      </section>

      <!-- Cache -->
      <section class="space-y-4">
        <div class="space-y-4 rounded-lg border p-4">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <Database class="text-muted-foreground h-5 w-5" />
              <div>
                <p class="text-sm font-medium">
                  {m.settings_storage_cache_title()}
                </p>
                <p class="text-muted-foreground text-xs">
                  {m.settings_storage_cache_description()}
                </p>
              </div>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onclick={loadData}
              disabled={clearing}
            >
              <RefreshCw class="h-4 w-4" />
            </Button>
          </div>

          {#if cache}
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm font-medium">
                  {m.settings_storage_cache_size({size: cache.sizeFormatted})}
                </p>
                <p class="text-muted-foreground text-xs">
                  {m.settings_storage_files_count({count: cache.fileCount})}
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                onclick={handleClearCache}
                disabled={clearing || cache.fileCount === 0}
              >
                {#if clearing}
                  <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                {:else}
                  <Trash2 class="mr-2 h-4 w-4" />
                {/if}
                {m.settings_storage_cache_clear()}
              </Button>
            </div>

            {#if clearSuccess}
              <p class="text-xs text-green-600 dark:text-green-400">
                {clearSuccess}
              </p>
            {/if}
          {:else}
            <p class="text-muted-foreground text-xs">
              {m.settings_storage_cache_unavailable()}
            </p>
          {/if}
        </div>
      </section>
    {/if}
  </div>
</div>
