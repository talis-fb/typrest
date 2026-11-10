<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    ApiRequestError,
    getTemplate,
    listTemplates,
    renderTemplate,
    type JsonValue,
    type TemplateMetadata,
    type TemplateSummary,
  } from "./api";
  import PayloadEditor from "./components/PayloadEditor.svelte";
  import PdfPreview from "./components/PdfPreview.svelte";
  import TemplateList from "./components/TemplateList.svelte";
  import logo from "../../brand/logos/typrest-logo-horizontal.svg";
  import logoInverse from "../../brand/logos/typrest-logo-horizontal-inverse.svg";

  let templates: TemplateSummary[] = [];
  let selected: TemplateMetadata | null = null;
  let loading = true;
  let loadingTemplate = false;
  let rendering = false;
  let pageError = "";
  let renderError: ApiRequestError | null = null;
  let blobUrl: string | null = null;
  let loadController: AbortController | null = null;
  let renderController: AbortController | null = null;

  onMount(() => {
    void initialize();
    window.addEventListener("popstate", handlePopstate);
  });

  onDestroy(() => {
    window.removeEventListener("popstate", handlePopstate);
    loadController?.abort();
    renderController?.abort();
    revokeBlob();
  });

  async function initialize() {
    await loadTemplates();
    const requested = new URL(window.location.href).searchParams.get("template");
    if (requested) await selectTemplate(requested, false);
  }

  function handlePopstate() {
    const requested = new URL(window.location.href).searchParams.get("template");
    if (requested) void selectTemplate(requested, false);
    else {
      selected = null;
      renderError = null;
      revokeBlob();
    }
  }

  async function loadTemplates() {
    loading = true;
    pageError = "";
    try {
      templates = await listTemplates();
    } catch (error) {
      pageError = error instanceof Error ? error.message : "Could not load templates";
    } finally {
      loading = false;
    }
  }

  async function selectTemplate(id: string, updateUrl = true) {
    loadController?.abort();
    const controller = new AbortController();
    loadController = controller;
    loadingTemplate = true;
    pageError = "";
    renderError = null;
    revokeBlob();

    try {
      selected = await getTemplate(id, controller.signal);
      if (updateUrl) {
        const url = new URL(window.location.href);
        url.searchParams.set("template", id);
        window.history.pushState({}, "", url);
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      selected = null;
      pageError = error instanceof Error ? error.message : `Could not open "${id}"`;
    } finally {
      if (loadController === controller) loadingTemplate = false;
    }
  }

  function closeTemplate() {
    renderController?.abort();
    selected = null;
    renderError = null;
    revokeBlob();
    const url = new URL(window.location.href);
    url.searchParams.delete("template");
    window.history.pushState({}, "", url);
  }

  async function render(payload: JsonValue) {
    if (!selected) return;
    renderController?.abort();
    const controller = new AbortController();
    renderController = controller;
    rendering = true;
    renderError = null;

    try {
      const blob = await renderTemplate(selected.id, payload, controller.signal);
      revokeBlob();
      blobUrl = URL.createObjectURL(blob);
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      renderError =
        error instanceof ApiRequestError
          ? error
          : new ApiRequestError("Unexpected error while rendering", null, null);
    } finally {
      if (renderController === controller) rendering = false;
    }
  }

  function revokeBlob() {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
    blobUrl = null;
  }
</script>

<svelte:head>
  <title>{selected ? `${selected.id} · Typrest` : "Typrest Web UI"}</title>
</svelte:head>

<header class="sticky top-0 z-30 border-b border-base-300 bg-base-100">
  <div class="mx-auto flex h-16 max-w-[1600px] items-center justify-between px-4 sm:px-6">
    <button
      type="button"
      class="flex items-center gap-3 rounded-field p-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      on:click={closeTemplate}
      aria-label="Typrest template list"
    >
      <img src={logo} alt="" height="28" width="109" class="h-7 w-auto dark:hidden" />
      <img src={logoInverse} alt="" height="28" width="109" class="hidden h-7 w-auto dark:block" />
      <span class="h-5 w-px bg-base-300" aria-hidden="true"></span>
      <span class="text-xs font-medium uppercase tracking-wider text-base-content/60">Web UI</span>
    </button>
    <span class="badge badge-outline gap-2 border-base-300 text-xs text-base-content/70">
      <span class="type-dot" aria-hidden="true"></span>Development only
    </span>
  </div>
</header>

<main>
  {#if loading || loadingTemplate}
    <div class="grid workspace-height place-items-center">
      <div class="text-center">
        <span class="loading loading-spinner loading-lg text-primary"></span>
        <p class="mt-3 text-sm text-base-content/60">
          {loadingTemplate ? "Opening template…" : "Loading templates…"}
        </p>
      </div>
    </div>
  {:else if pageError}
    <div class="mx-auto max-w-2xl px-4 py-16">
      <div class="alert alert-error" role="alert">
        <div>
          <h1 class="font-display-brand text-lg">The Web UI could not continue</h1>
          <p class="mt-1 text-sm">{pageError}</p>
          <button type="button" class="btn btn-sm mt-4" on:click={loadTemplates}>Try again</button>
        </div>
      </div>
    </div>
  {:else if selected}
    <div class="mx-auto max-w-[1600px] px-3 py-4 sm:px-5">
      <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-3">
          <button type="button" class="btn btn-ghost btn-sm" on:click={closeTemplate}>← Templates</button>
          <span class="text-base-content/30">/</span>
          <h1 class="font-mono text-lg font-normal">{selected.id}</h1>
          <span class="badge badge-sm badge-outline border-base-300 text-base-content/70">{selected.schema ? "Schema v1" : "No schema"}</span>
        </div>
        <code class="rounded-field border border-base-300 bg-base-100 px-2 py-1 font-mono text-xs">POST /{selected.id}</code>
      </div>

      <div class="workspace-height grid gap-4 lg:grid-cols-[minmax(22rem,0.9fr)_minmax(28rem,1.35fr)]">
        {#key selected.id}
          <PayloadEditor template={selected} {rendering} on:render={(event) => render(event.detail)} />
        {/key}
        <PdfPreview
          templateId={selected.id}
          {blobUrl}
          {rendering}
          error={renderError}
        />
      </div>
    </div>
  {:else}
    <TemplateList {templates} on:select={(event) => selectTemplate(event.detail)} />
  {/if}
</main>
