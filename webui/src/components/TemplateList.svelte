<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { TemplateSummary } from "../api";

  export let templates: TemplateSummary[] = [];
  const dispatch = createEventDispatcher<{ select: string }>();
</script>

<section aria-labelledby="templates-heading" class="mx-auto w-full max-w-6xl px-4 py-10 sm:px-6">
  <div class="mb-8 max-w-2xl">
    <div class="badge badge-primary badge-outline mb-3">Developer workspace</div>
    <h1 id="templates-heading" class="text-3xl font-bold tracking-tight sm:text-4xl">
      Inspect and render templates
    </h1>
    <p class="mt-3 text-base-content/65">
      Choose a prepared template, shape a test payload, and preview the generated PDF.
    </p>
  </div>

  {#if templates.length === 0}
    <div class="card border border-base-300 bg-base-100 shadow-sm">
      <div class="card-body items-center py-16 text-center">
        <h2 class="card-title">No templates are prepared</h2>
        <p class="text-base-content/60">
          Add a template and refresh this page after Papelon has loaded it.
        </p>
      </div>
    </div>
  {:else}
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {#each templates as template}
        <button
          type="button"
          class="card border border-base-300 bg-base-100 text-left shadow-sm transition hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          on:click={() => dispatch("select", template.name)}
          aria-label={`Open ${template.name} template`}
        >
          <span class="card-body">
            <span class="flex items-start justify-between gap-4">
              <span class="grid size-10 place-items-center rounded-xl bg-primary/10 font-mono text-sm font-bold text-primary">
                {template.name.slice(0, 2).toUpperCase()}
              </span>
              <span class:badge-success={template.hasSchema} class="badge badge-sm badge-ghost">
                {template.hasSchema ? "Schema" : "Raw JSON"}
              </span>
            </span>
            <span class="mt-4 font-mono text-lg font-semibold">{template.name}</span>
            <span class="text-sm text-base-content/55">Open template workspace</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}
</section>
