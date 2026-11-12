<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { TemplateSummary } from "../api";

  export let templates: TemplateSummary[] = [];
  const dispatch = createEventDispatcher<{ select: string }>();
</script>

<section aria-labelledby="templates-heading" class="mx-auto w-full max-w-6xl px-4 py-10 sm:px-6">
  <div class="mb-8 max-w-2xl">
    <p class="mb-3 inline-flex items-center gap-2 text-sm font-medium uppercase tracking-wider text-base-content/60">
      <span class="type-dot" aria-hidden="true"></span>
      Developer workspace
    </p>
    <h1 id="templates-heading" class="font-display-brand text-4xl leading-tight sm:text-5xl">
      Inspect and render templates
    </h1>
    <p class="mt-4 text-lg text-base-content/70">
      Choose a prepared template, shape a test payload, and preview the generated PDF.
    </p>
  </div>

  {#if templates.length === 0}
    <div class="card card-border border-base-300 bg-base-100">
      <div class="card-body items-center py-16 text-center">
        <h2 class="font-display-brand text-2xl">No templates are prepared</h2>
        <p class="text-base-content/70">
          Add a template and refresh this page after Typrest has loaded it.
        </p>
      </div>
    </div>
  {:else}
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {#each templates as template}
        <button
          type="button"
          class="group card card-border border-base-300 bg-base-100 text-left transition-colors hover:border-base-content/40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          on:click={() => dispatch("select", template.id)}
          aria-label={`Open ${template.id} template`}
        >
          <span class="card-body">
            <span class="flex items-start justify-between gap-4">
              <span class="grid size-10 place-items-center rounded-[3px] bg-primary font-mono text-sm text-primary-content">
                {template.id.split("/").at(-1)?.slice(0, 2).toUpperCase()}
              </span>
              <span
                class:badge-success={template.hasSchema}
                class:badge-soft={template.hasSchema}
                class:badge-outline={!template.hasSchema}
                class:border-base-300={!template.hasSchema}
                class:text-base-content={!template.hasSchema}
                class="badge badge-sm"
              >
                {template.hasSchema ? "Schema" : "Raw JSON"}
              </span>
            </span>
            <span class="mt-4 font-mono text-lg font-normal group-hover:text-accent">{template.id}</span>
            <span class="text-sm text-base-content/60">Open template workspace →</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}
</section>
