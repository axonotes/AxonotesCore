<script lang="ts">
  import {activeProfile} from "$lib/stores/authStore";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card";
  import {Badge} from "$lib/components/ui/badge";
  import * as m from "$lib/paraglide/messages.js";
</script>

<div class="space-y-6">
  <div>
    <h2 class="text-3xl font-bold tracking-tight">{m.dashboard_title()}</h2>
    <p class="text-muted-foreground">
      {m.dashboard_welcome({name: $activeProfile?.name || "User"})}
    </p>
  </div>

  <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
    <Card>
      <CardHeader>
        <CardTitle>{m.profile_info_title()}</CardTitle>
        <CardDescription>{m.profile_info_description()}</CardDescription>
      </CardHeader>
      <CardContent class="space-y-2">
        <div>
          <p class="text-sm font-medium">{m.profile_name_label()}</p>
          <p class="text-muted-foreground text-sm">{$activeProfile?.name}</p>
        </div>
        <div>
          <p class="text-sm font-medium">{m.profile_email_label()}</p>
          <p class="text-muted-foreground text-sm">{$activeProfile?.email}</p>
        </div>
        <div>
          <p class="text-sm font-medium">{m.profile_status_label()}</p>
          <Badge
            variant="outline"
            class="border-green-500/20 bg-green-500/10 text-green-700"
          >
            {m.profile_status_active()}
          </Badge>
        </div>
      </CardContent>
    </Card>
  </div>
</div>
