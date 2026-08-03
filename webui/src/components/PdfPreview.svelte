<script lang="ts">
  import type { ApiRequestError } from "../api";
  import { templatePdfFilename } from "../api";

  export let templateId: string;
  export let blobUrl: string | null = null;
  export let rendering = false;
  export let error: ApiRequestError | null = null;
</script>

<section class="card h-full border border-base-300 bg-base-100 shadow-sm">
  <div class="card-body gap-4 p-4 sm:p-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 class="card-title text-lg">PDF preview</h2>
        <p class="text-xs text-base-content/55">Latest successful render</p>
      </div>
      {#if blobUrl}
        <a
          class="btn btn-outline btn-sm"
          href={blobUrl}
          download={templatePdfFilename(templateId)}
        >
          Download PDF
        </a>
      {/if}
    </div>

    {#if error}
      <div class="alert alert-error items-start" role="alert" aria-live="polite">
        <div class="min-w-0">
          <h3 class="font-semibold">{error.message}</h3>
          {#if error.status}
            <p class="mt-1 text-xs opacity-75">HTTP {error.status} · {error.body?.error ?? "request_failed"}</p>
          {/if}
          {#if error.body?.violations?.length}
            <ul class="mt-3 list-disc space-y-1 pl-5 text-sm">
              {#each error.body.violations as violation}
                <li><code>{violation.field}</code>: {violation.issue}</li>
              {/each}
            </ul>
          {/if}
          {#if error.body?.diagnostics?.length}
            <div class="mt-3 space-y-2">
              {#each error.body.diagnostics as diagnostic}
                <div class="rounded-lg bg-error-content/10 p-2 text-sm">
                  <p class="font-medium">{diagnostic.message}</p>
                  {#if diagnostic.location}
                    <p class="mt-1 font-mono text-xs opacity-75">
                      {diagnostic.location.path}:{diagnostic.location.range.start.line}:{diagnostic.location.range.start.column}
                    </p>
                  {/if}
                  {#if diagnostic.hints?.length}
                    {#each diagnostic.hints as hint}
                      <p class="mt-1 text-xs opacity-75">Hint: {hint.message}</p>
                    {/each}
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <div class="relative min-h-0 flex-1 overflow-hidden rounded-xl border border-base-300 bg-base-200">
      {#if rendering}
        <div class="absolute inset-0 z-10 grid place-items-center bg-base-100/80 backdrop-blur-sm">
          <div class="text-center">
            <span class="loading loading-spinner loading-lg text-primary"></span>
            <p class="mt-3 text-sm font-medium">Compiling PDF…</p>
          </div>
        </div>
      {/if}

      {#if blobUrl}
        <iframe
          class="pdf-frame h-full w-full bg-white"
          src={blobUrl}
          title={`PDF preview for ${templateId}`}
        ></iframe>
      {:else}
        <div class="pdf-frame grid place-items-center p-8 text-center">
          <div class="max-w-xs">
            <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-primary/10 font-mono text-xl font-bold text-primary">
              PDF
            </div>
            <h3 class="mt-4 font-semibold">Ready for a test render</h3>
            <p class="mt-2 text-sm text-base-content/55">
              Edit the payload and render when you are ready. The preview will appear here.
            </p>
          </div>
        </div>
      {/if}
    </div>
  </div>
</section>
