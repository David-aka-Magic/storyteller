<!-- src/components/settings/AboutSection.svelte — Static about text -->
<div class="about-section">
  <p class="about-text">
    <strong>AI Story Writer</strong> v0.1.0<br />
    An interactive narrative experience powered by local AI.<br /><br />
    Built with <strong>Tauri</strong>, <strong>Svelte</strong>, and <strong>Rust</strong>.<br />
    Image generation via <strong>Stable Diffusion</strong> (A1111 / ComfyUI).<br />
    Text generation via <strong>Ollama</strong>.
  </p>
</div>

<style>
  .about-section {
    padding: 10px 0;
  }

  .about-text {
    color: var(--text-secondary);
    font-size: 0.9em;
    line-height: 1.8;
    margin: 0;
  }
</style>
