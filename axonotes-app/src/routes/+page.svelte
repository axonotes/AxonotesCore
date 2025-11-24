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
</script>

<div class="space-y-6">
  <div>
    <h2 class="text-3xl font-bold tracking-tight">Dashboard</h2>
    <p class="text-muted-foreground">
      Welcome back, {$activeProfile?.name || "User"}!
    </p>
  </div>

  <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
    <Card>
      <CardHeader>
        <CardTitle>Profile Info</CardTitle>
        <CardDescription>Your account details</CardDescription>
      </CardHeader>
      <CardContent class="space-y-2">
        <div>
          <p class="text-sm font-medium">Name</p>
          <p class="text-muted-foreground text-sm">{$activeProfile?.name}</p>
        </div>
        <div>
          <p class="text-sm font-medium">Email</p>
          <p class="text-muted-foreground text-sm">{$activeProfile?.email}</p>
        </div>
        <div>
          <p class="text-sm font-medium">Status</p>
          <Badge
            variant="outline"
            class="border-green-500/20 bg-green-500/10 text-green-700"
          >
            Active
          </Badge>
        </div>
      </CardContent>
    </Card>
  </div>
</div>
