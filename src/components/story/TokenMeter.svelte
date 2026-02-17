<!-- src/components/story/TokenMeter.svelte -->
<!--
  Visual token usage meter from compression diagnostics.
  Shows how close the context is to the compression threshold.
-->
<script lang="ts">
  import type { OrchestratorCompressionInfo } from '$lib/orchestrator-types';

  export let info: OrchestratorCompressionInfo | null = null;
  export let compact: boolean = false;

  $: usedTokens = info?.estimated_total_tokens ?? 0;
  $: maxTokens = info?.max_context_tokens ?? 8192;
  $: threshold = info?.compression_threshold ?? 6000;
  $: percentage = maxTokens > 0 ? Math.min((usedTokens / maxTokens) * 100, 100) : 0;
  $: thresholdPct = maxTokens > 0 ? (threshold / maxTokens) * 100 : 73;
  $: isNearLimit = info?.needs_compression ?? false;
  $: isWarning = percentage > 60;
  $: isDanger = percentage > 85;

  $: fillColor = isDanger ? '#e06060' : isWarning ? '#e9a84c' : '#4caf82';
  $: label = compact
    ? `${Math.round(percentage)}%`
    : `${usedTokens.toLocaleString()} / ${maxTokens.toLocaleString()}`;
</script>

{#if info}
  <div class="token-meter" class:compact class:near-limit={isNearLimit}>
    {#if !compact}
      <div class="meter-header">
        <span class="meter-label">Context</span>
        <span class="meter-value" class:warning={isWarning} class:danger={isDanger}>{label}</span>
      </div>
    {/if}

    <div class="meter-track">
      <div
        class="meter-fill"
        style="width: {percentage}%; background: {fillColor};"
      ></div>
      <div
        class="meter-threshold"
        style="left: {thresholdPct}%;"
        title="Compression threshold"
      ></div>
    </div>

    {#if !compact && info}
      <div class="meter-details">
        <span class="detail-item">
          {info.compressed_turns} compressed
        </span>
        <span class="detail-sep">·</span>
        <span class="detail-item">
          {info.recent_turns} recent
        </span>
        {#if isNearLimit}
          <span class="detail-sep">·</span>
          <span class="detail-item compress-warning">compressing soon</span>
        {/if}
      </div>
    {/if}

    {#if compact}
      <span class="compact-label" class:warning={isWarning} class:danger={isDanger}>{label}</span>
    {/if}
  </div>
{/if}

<style>
  .token-meter {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 120px;
  }

  .token-meter.compact {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    min-width: auto;
  }

  .meter-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .meter-label {
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted, #6e7681);
  }

  .meter-value {
    font-size: 0.72rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-secondary, #8b949e);
  }

  .meter-value.warning { color: #e9a84c; }
  .meter-value.danger { color: #e06060; }

  .meter-track {
    position: relative;
    height: 4px;
    border-radius: 2px;
    background: var(--story-meter-bg, rgba(255, 255, 255, 0.06));
    overflow: visible;
  }

  .compact .meter-track {
    flex: 1;
    min-width: 60px;
    height: 3px;
  }

  .meter-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.4s ease, background 0.3s ease;
  }

  .meter-threshold {
    position: absolute;
    top: -2px;
    width: 1px;
    height: 8px;
    background: rgba(255, 255, 255, 0.25);
    transform: translateX(-50%);
  }

  .compact .meter-threshold {
    top: -1px;
    height: 5px;
  }

  .meter-details {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.65rem;
    color: var(--text-muted, #6e7681);
  }

  .detail-sep {
    opacity: 0.4;
  }

  .compress-warning {
    color: #e9a84c;
    font-style: italic;
  }

  .compact-label {
    font-size: 0.68rem;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: var(--text-muted, #6e7681);
    white-space: nowrap;
  }

  .compact-label.warning { color: #e9a84c; }
  .compact-label.danger { color: #e06060; }

  .near-limit .meter-track {
    box-shadow: 0 0 6px rgba(233, 168, 76, 0.2);
  }
</style>