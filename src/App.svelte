<script lang="ts">
  import Main from './components/Simplified.svelte';
  import Sticky from './components/Sticky.svelte';
  import QuickCapture from './components/QuickCapture.svelte';
  const params = new URLSearchParams(window.location.search);
  const stickyId = params.get('sticky');
  const isCapture = params.has('capture');
</script>

{#if isCapture}
  <QuickCapture />
{:else if stickyId}
  <Sticky id={stickyId} />
{:else}
  <Main />
{/if}
