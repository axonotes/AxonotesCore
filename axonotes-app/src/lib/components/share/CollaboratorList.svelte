<script lang="ts">
  import {onMount, onDestroy} from "svelte";
  import {
    MoreHorizontal,
    Crown,
    Pencil,
    Eye,
    UserMinus,
    ArrowRightLeft,
    Loader2,
  } from "@lucide/svelte";
  import {Badge} from "$lib/components/ui/badge";
  import {Button} from "$lib/components/ui/button";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import * as Dialog from "$lib/components/ui/dialog";
  import {
    ShareService,
    type Collaborator,
    type ShareRole,
  } from "$lib/services/share";
  import {getCurrentIdentityHex} from "$lib/services/block";
  import * as m from "$lib/paraglide/messages.js";
  import type {UnlistenFn} from "@tauri-apps/api/event";

  interface Props {
    docId: string;
  }

  let {docId}: Props = $props();

  let collaborators = $state<Collaborator[]>([]);
  let currentUserId = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let actionLoading = $state<string | null>(null);

  // Confirmation dialog state
  let confirmDialog = $state<{
    open: boolean;
    type: "remove" | "transfer" | null;
    userId: string;
  }>({open: false, type: null, userId: ""});

  let unlisteners: UnlistenFn[] = [];

  // Derived: current user's role
  let currentUserRole = $derived(
    collaborators.find((c) => c.userId === currentUserId)?.role ?? null
  );

  // Can manage collaborators if owner or editor
  let canManage = $derived(
    currentUserRole === "owner" || currentUserRole === "editor"
  );

  async function loadCollaborators() {
    try {
      loading = true;
      error = null;
      collaborators = await ShareService.getCollaborators(docId);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function handleRoleChange(
    userId: string,
    newRole: "editor" | "reader"
  ) {
    actionLoading = userId;
    try {
      await ShareService.updateUserRole(docId, userId, newRole);
      await loadCollaborators();
    } catch (e) {
      console.error("Failed to change role:", e);
    } finally {
      actionLoading = null;
    }
  }

  async function handleRemove() {
    if (!confirmDialog.userId) return;
    actionLoading = confirmDialog.userId;
    try {
      await ShareService.removeUser(docId, confirmDialog.userId);
      await loadCollaborators();
    } catch (e) {
      console.error("Failed to remove user:", e);
    } finally {
      actionLoading = null;
      confirmDialog = {open: false, type: null, userId: ""};
    }
  }

  async function handleTransferOwnership() {
    if (!confirmDialog.userId) return;
    actionLoading = confirmDialog.userId;
    try {
      await ShareService.transferOwnership(docId, confirmDialog.userId);
      await loadCollaborators();
    } catch (e) {
      console.error("Failed to transfer ownership:", e);
    } finally {
      actionLoading = null;
      confirmDialog = {open: false, type: null, userId: ""};
    }
  }

  function showRemoveConfirm(userId: string) {
    confirmDialog = {open: true, type: "remove", userId};
  }

  function showTransferConfirm(userId: string) {
    confirmDialog = {open: true, type: "transfer", userId};
  }

  function truncateUserId(userId: string): string {
    if (userId.length <= 12) return userId;
    return `${userId.slice(0, 6)}...${userId.slice(-4)}`;
  }

  function getRoleBadgeVariant(
    role: ShareRole
  ): "default" | "secondary" | "outline" {
    switch (role) {
      case "owner":
        return "default";
      case "editor":
        return "secondary";
      case "reader":
        return "outline";
    }
  }

  function getRoleIcon(role: ShareRole) {
    switch (role) {
      case "owner":
        return Crown;
      case "editor":
        return Pencil;
      case "reader":
        return Eye;
    }
  }

  onMount(async () => {
    currentUserId = await getCurrentIdentityHex();
    await loadCollaborators();

    // Listen for collaborator changes
    unlisteners.push(
      await ShareService.onCollaboratorAdded(() => loadCollaborators()),
      await ShareService.onCollaboratorRemoved(() => loadCollaborators()),
      await ShareService.onCollaboratorRoleChanged(() => loadCollaborators()),
      await ShareService.onOwnershipTransferred(() => loadCollaborators())
    );
  });

  onDestroy(() => {
    unlisteners.forEach((unlisten) => unlisten());
  });
</script>

<div class="space-y-3">
  <h4 class="text-sm font-medium">{m.share_collaborators_title()}</h4>

  {#if loading}
    <div class="flex items-center justify-center py-4">
      <Loader2 class="text-muted-foreground h-5 w-5 animate-spin" />
    </div>
  {:else if error}
    <p class="text-destructive text-sm">{error}</p>
  {:else if collaborators.length === 0}
    <p class="text-muted-foreground text-sm">{m.share_collaborators_empty()}</p>
  {:else}
    <div class="space-y-2">
      {#each collaborators as collab (collab.userId)}
        {@const isCurrentUser = collab.userId === currentUserId}
        {@const RoleIcon = getRoleIcon(collab.role)}
        <div
          class="hover:bg-accent/50 flex items-center justify-between rounded-lg p-2 transition-colors"
        >
          <div class="flex min-w-0 items-center gap-3">
            <div
              class="bg-primary/10 flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-xs font-medium"
            >
              {collab.userId.slice(0, 2).toUpperCase()}
            </div>
            <div class="min-w-0">
              <p class="truncate text-sm font-medium">
                {truncateUserId(collab.userId)}
                {#if isCurrentUser}
                  <span class="text-muted-foreground font-normal">
                    {m.share_collaborator_you()}
                  </span>
                {/if}
              </p>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <Badge variant={getRoleBadgeVariant(collab.role)}>
              <RoleIcon class="mr-1 h-3 w-3" />
              {collab.role === "owner"
                ? m.share_role_owner()
                : collab.role === "editor"
                  ? m.share_role_editor()
                  : m.share_role_reader()}
            </Badge>

            {#if canManage && !isCurrentUser && collab.role !== "owner"}
              <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                  {#snippet child({props})}
                    <Button
                      {...props}
                      variant="ghost"
                      size="icon"
                      class="h-8 w-8"
                      disabled={actionLoading === collab.userId}
                      aria-label={m.share_change_role()}
                    >
                      {#if actionLoading === collab.userId}
                        <Loader2 class="h-4 w-4 animate-spin" />
                      {:else}
                        <MoreHorizontal class="h-4 w-4" />
                      {/if}
                    </Button>
                  {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end">
                  <DropdownMenu.Label
                    >{m.share_change_role()}</DropdownMenu.Label
                  >
                  <DropdownMenu.Separator />
                  <DropdownMenu.Item
                    onclick={() => handleRoleChange(collab.userId, "editor")}
                    class={collab.role === "editor" ? "bg-accent" : ""}
                  >
                    <Pencil class="mr-2 h-4 w-4" />
                    {m.share_role_editor()}
                  </DropdownMenu.Item>
                  <DropdownMenu.Item
                    onclick={() => handleRoleChange(collab.userId, "reader")}
                    class={collab.role === "reader" ? "bg-accent" : ""}
                  >
                    <Eye class="mr-2 h-4 w-4" />
                    {m.share_role_reader()}
                  </DropdownMenu.Item>
                  <DropdownMenu.Separator />
                  {#if currentUserRole === "owner"}
                    <DropdownMenu.Item
                      onclick={() => showTransferConfirm(collab.userId)}
                    >
                      <ArrowRightLeft class="mr-2 h-4 w-4" />
                      {m.share_transfer_ownership()}
                    </DropdownMenu.Item>
                  {/if}
                  <DropdownMenu.Item
                    class="text-destructive focus:text-destructive"
                    onclick={() => showRemoveConfirm(collab.userId)}
                  >
                    <UserMinus class="mr-2 h-4 w-4" />
                    {m.share_remove_user()}
                  </DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Root>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Confirmation Dialog -->
<Dialog.Root bind:open={confirmDialog.open}>
  <Dialog.Content class="max-w-xs">
    <Dialog.Header>
      <Dialog.Title>
        {#if confirmDialog.type === "remove"}
          {m.share_remove_user()}
        {:else}
          {m.share_transfer_ownership()}
        {/if}
      </Dialog.Title>
      <Dialog.Description>
        {#if confirmDialog.type === "remove"}
          {m.share_remove_confirm({name: truncateUserId(confirmDialog.userId)})}
        {:else}
          {m.share_transfer_confirm({
            name: truncateUserId(confirmDialog.userId),
          })}
        {/if}
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="flex-col gap-2 sm:flex-col">
      <Button
        variant="outline"
        class="w-full"
        onclick={() => (confirmDialog = {open: false, type: null, userId: ""})}
      >
        {m.common_button_cancel()}
      </Button>
      <Button
        variant={confirmDialog.type === "remove" ? "destructive" : "default"}
        class="w-full"
        onclick={confirmDialog.type === "remove"
          ? handleRemove
          : handleTransferOwnership}
        disabled={!!actionLoading}
      >
        {#if actionLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {/if}
        {m.common_button_confirm()}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
