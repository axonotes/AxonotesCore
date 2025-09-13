<script lang="ts">
    export let data;
</script>

<h1 class="text-xl">Welcome to your Dashboard, {data.user.firstName}!</h1>
<p>You are logged in with the email: {data.user.email}</p>

<pre>{JSON.stringify(data.user, null, 2)}</pre>
