<!-- src/routes/+layout.svelte -->
<script>
  import '../app.css';
</script>

<div class="app-root">
  <slot />
</div>

<style>
  .app-root {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>