<script lang="ts">
  import {
    profilesState,
    activeProfile,
    switchProfile,
    logout,
    startLogin,
  } from "$lib/stores/authStore";
  import {Button} from "$lib/components/ui/button";
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
    DropdownMenuGroup,
  } from "$lib/components/ui/dropdown-menu";
  import {Avatar, AvatarFallback} from "$lib/components/ui/avatar";
  import {Check, ChevronDown, LogOut, Plus, User} from "@lucide/svelte";

  function getInitials(name: string): string {
    return name
      .split(" ")
      .map((n) => n[0])
      .join("")
      .toUpperCase()
      .slice(0, 2);
  }

  async function handleSwitchProfile(profileId: string) {
    await switchProfile(profileId);
  }

  async function handleLogout(profileId: string) {
    await logout(profileId);
  }

  async function handleAddAccount() {
    await startLogin();
  }
</script>

<DropdownMenu>
  <DropdownMenuTrigger>
    <Button variant="outline" class="w-full justify-between">
      <div class="flex items-center gap-2 overflow-hidden">
        {#if $activeProfile}
          <Avatar class="h-6 w-6">
            <AvatarFallback class="text-xs">
              {getInitials($activeProfile.name)}
            </AvatarFallback>
          </Avatar>
          <div class="flex flex-col items-start overflow-hidden">
            <span class="truncate text-sm font-medium"
              >{$activeProfile.name}</span
            >
            <span class="text-muted-foreground truncate text-xs"
              >{$activeProfile.email}</span
            >
          </div>
        {:else}
          <User class="h-4 w-4" />
          <span>No account selected</span>
        {/if}
      </div>
      <ChevronDown class="h-4 w-4 flex-shrink-0 opacity-50" />
    </Button>
  </DropdownMenuTrigger>
  <DropdownMenuContent class="w-64" align="end">
    <DropdownMenuLabel>Accounts</DropdownMenuLabel>
    <DropdownMenuSeparator />

    <DropdownMenuGroup>
      {#each $profilesState.profiles as profile (profile.id)}
        <DropdownMenuItem
          class="flex items-center gap-2"
          onclick={() => handleSwitchProfile(profile.id)}
        >
          <Avatar class="h-6 w-6">
            <AvatarFallback class="text-xs">
              {getInitials(profile.name)}
            </AvatarFallback>
          </Avatar>
          <div class="flex flex-1 flex-col overflow-hidden">
            <span class="truncate text-sm font-medium">{profile.name}</span>
            <span class="text-muted-foreground truncate text-xs"
              >{profile.email}</span
            >
          </div>
          {#if profile.id === $activeProfile?.id}
            <Check class="h-4 w-4" />
          {/if}
        </DropdownMenuItem>
      {/each}
    </DropdownMenuGroup>

    <DropdownMenuSeparator />

    <DropdownMenuItem onclick={handleAddAccount}>
      <Plus class="mr-2 h-4 w-4" />
      <span>Add another account</span>
    </DropdownMenuItem>

    {#if $activeProfile}
      <DropdownMenuSeparator />
      <DropdownMenuItem
        class="text-destructive focus:text-destructive"
        onclick={() => handleLogout($activeProfile.id)}
      >
        <LogOut class="mr-2 h-4 w-4" />
        <span>Sign out</span>
      </DropdownMenuItem>
    {/if}
  </DropdownMenuContent>
</DropdownMenu>
