<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { JsonValue, TemplateMetadata } from "../api";
  import {
    composeFormPayload,
    hydrateObjectForForm,
    isJsonObject,
    seedObject,
    splitRawObject,
    type CustomField,
    type CustomFieldIssue,
  } from "../payload";
  import SchemaFieldEditor from "./SchemaFieldEditor.svelte";

  export let template: TemplateMetadata;
  export let rendering = false;

  const dispatch = createEventDispatcher<{ render: JsonValue }>();
  const root = template.schema?.request.body;
  const formAvailable = root?.type === "object";
  const schemaFields = root?.properties ?? {};

  let mode: "form" | "raw" = formAvailable ? "form" : "raw";
  let formPayload = formAvailable ? seedObject(root) : {};
  let rawText = JSON.stringify(formPayload, null, 2);
  let rawError = "";
  let modeError = "";
  let customFields: CustomField[] = [];
  let customIssues: CustomFieldIssue[] = [];
  let nextCustomId = 1;
  let fieldIssues: Record<string, string> = {};

  function switchMode(next: "form" | "raw") {
    modeError = "";
    if (next === mode) return;

    if (next === "raw") {
      if (Object.keys(fieldIssues).length > 0) {
        modeError = "Resolve field errors before switching to Raw JSON.";
        return;
      }
      const composed = composeFormPayload(formPayload, customFields, schemaFields);
      customIssues = composed.issues;
      if (!composed.payload) {
        modeError = "Resolve custom field errors before switching to Raw JSON.";
        return;
      }
      rawText = JSON.stringify(composed.payload, null, 2);
      rawError = "";
      mode = "raw";
      return;
    }

    try {
      const parsed = JSON.parse(rawText) as JsonValue;
      if (!isJsonObject(parsed)) {
        modeError = "Form mode requires a JSON object at the payload root.";
        return;
      }
      const split = splitRawObject(parsed, schemaFields);
      formPayload = hydrateObjectForForm(split.known, root!);
      customFields = split.custom;
      nextCustomId = Math.max(1, ...customFields.map((field) => field.id + 1));
      customIssues = [];
      fieldIssues = {};
      rawError = "";
      mode = "form";
    } catch (error) {
      rawError = error instanceof Error ? error.message : "Invalid JSON";
      modeError = "Fix the JSON before switching to Form mode.";
    }
  }

  function updateRaw(text: string) {
    rawText = text;
    try {
      JSON.parse(text);
      rawError = "";
    } catch (error) {
      rawError = error instanceof Error ? error.message : "Invalid JSON";
    }
  }

  function updateKnown(
    name: string,
    detail: { included: boolean; value: JsonValue | undefined },
  ) {
    formPayload = { ...formPayload };
    if (detail.included && detail.value !== undefined) {
      formPayload[name] = detail.value;
    } else {
      delete formPayload[name];
    }
  }

  function updateValidity(path: string, issue: string | null) {
    fieldIssues = { ...fieldIssues };
    if (issue) fieldIssues[path] = issue;
    else delete fieldIssues[path];
  }

  function addCustomField() {
    customFields = [
      ...customFields,
      { id: nextCustomId++, key: "", valueText: '""' },
    ];
    validateCustomFields();
  }

  function updateCustomField(id: number, patch: Partial<CustomField>) {
    customFields = customFields.map((field) =>
      field.id === id ? { ...field, ...patch } : field,
    );
    validateCustomFields();
  }

  function removeCustomField(id: number) {
    customFields = customFields.filter((field) => field.id !== id);
    validateCustomFields();
  }

  function validateCustomFields() {
    customIssues = composeFormPayload(formPayload, customFields, schemaFields).issues;
  }

  function customIssue(id: number): string | undefined {
    return customIssues.find((issue) => issue.id === id)?.issue;
  }

  function render() {
    modeError = "";
    if (mode === "raw") {
      try {
        const payload = JSON.parse(rawText) as JsonValue;
        rawError = "";
        dispatch("render", payload);
      } catch (error) {
        rawError = error instanceof Error ? error.message : "Invalid JSON";
      }
      return;
    }

    const composed = composeFormPayload(formPayload, customFields, schemaFields);
    customIssues = composed.issues;
    if (Object.keys(fieldIssues).length > 0 || !composed.payload) {
      modeError = "Resolve the highlighted payload errors before rendering.";
      return;
    }
    dispatch("render", composed.payload);
  }

  function handleKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      if (!rendering) render();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<section class="card card-border h-full border-base-300 bg-base-100">
  <div class="card-body gap-4 p-4 sm:p-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 class="font-display-brand text-xl">Payload</h2>
        <p class="text-xs text-base-content/60">Sent unchanged to <code class="font-mono">sys.inputs.data</code></p>
      </div>
      <div class="tabs tabs-box" role="tablist" aria-label="Payload input mode">
        <button
          type="button"
          class:tab-active={mode === "form"}
          class="tab"
          role="tab"
          aria-selected={mode === "form"}
          disabled={!formAvailable}
          title={formAvailable ? "Edit with schema controls" : "Form mode requires an object schema"}
          on:click={() => switchMode("form")}
        >Form</button>
        <button
          type="button"
          class:tab-active={mode === "raw"}
          class="tab"
          role="tab"
          aria-selected={mode === "raw"}
          on:click={() => switchMode("raw")}
        >Raw JSON</button>
      </div>
    </div>

    {#if !template.schema}
      <div class="alert alert-info py-3 text-sm">
        <span>This template has no schema. Raw JSON accepts any valid JSON value.</span>
      </div>
    {:else if !formAvailable}
      <div class="alert alert-info py-3 text-sm">
        <span>The schema root is not an object, so this template uses Raw JSON mode.</span>
      </div>
    {/if}

    {#if modeError}
      <div class="alert alert-error py-3 text-sm" role="alert" aria-live="polite">
        <span>{modeError}</span>
      </div>
    {/if}

    <div class="min-h-0 flex-1 overflow-auto pr-1">
      {#if mode === "form" && root}
        <div class="space-y-3">
          {#each Object.entries(schemaFields) as [name, field]}
            <SchemaFieldEditor
              {name}
              path={`$.${name}`}
              {field}
              value={formPayload[name]}
              included={field.required || Object.prototype.hasOwnProperty.call(formPayload, name)}
              onValidity={updateValidity}
              on:change={(event) => updateKnown(name, event.detail)}
            />
          {/each}

          <div class="divider my-4 text-xs font-medium tracking-wider text-base-content/60">CUSTOM TEST FIELDS</div>

          {#each customFields as custom (custom.id)}
            <fieldset class="rounded-box border border-base-300 p-3">
              <legend class="px-1 text-xs font-semibold text-base-content/70">Custom field</legend>
              <div class="grid gap-2 sm:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)_auto]">
                <label class="form-control">
                  <span class="label-text mb-1 text-xs">Key</span>
                  <input
                    class:input-error={customIssue(custom.id)}
                    class="input input-bordered input-sm w-full font-mono"
                    value={custom.key}
                    on:input={(event) => updateCustomField(custom.id, { key: event.currentTarget.value })}
                  />
                </label>
                <label class="form-control">
                  <span class="label-text mb-1 text-xs">JSON value</span>
                  <input
                    class:input-error={customIssue(custom.id)}
                    class="input input-bordered input-sm w-full font-mono"
                    value={custom.valueText}
                    on:input={(event) => updateCustomField(custom.id, { valueText: event.currentTarget.value })}
                  />
                </label>
                <button
                  type="button"
                  class="btn btn-ghost btn-sm self-end text-error"
                  aria-label={`Remove custom field ${custom.key || custom.id}`}
                  on:click={() => removeCustomField(custom.id)}
                >Remove</button>
              </div>
              {#if customIssue(custom.id)}
                <p class="mt-2 text-xs text-error" role="alert">{customIssue(custom.id)}</p>
              {/if}
            </fieldset>
          {/each}

          <button type="button" class="btn btn-ghost btn-sm border-dashed border-base-300" on:click={addCustomField}>
            + Add custom field
          </button>
        </div>
      {:else}
        <label class="form-control h-full">
          <span class="label-text mb-2 text-sm font-medium">Complete JSON payload</span>
          <textarea
            class:textarea-error={rawError}
            class="textarea textarea-bordered min-h-[28rem] w-full flex-1 resize-y font-mono text-sm leading-6"
            aria-describedby={rawError ? "raw-json-error" : undefined}
            spellcheck="false"
            value={rawText}
            on:input={(event) => updateRaw(event.currentTarget.value)}
          ></textarea>
        </label>
        {#if rawError}
          <p id="raw-json-error" class="mt-2 text-sm text-error" role="alert" aria-live="polite">
            Invalid JSON: {rawError}
          </p>
        {/if}
      {/if}
    </div>

    <div class="flex flex-wrap items-center justify-between gap-3 border-t border-base-300 pt-4">
      <span class="text-xs text-base-content/60">
        <kbd class="kbd kbd-xs">Ctrl</kbd>/<kbd class="kbd kbd-xs">⌘</kbd>
        <span class="mx-1">+</span><kbd class="kbd kbd-xs">Enter</kbd>
      </span>
      <button type="button" class="btn btn-primary min-w-32" disabled={rendering} on:click={render}>
        {#if rendering}<span class="loading loading-spinner loading-sm"></span>{/if}
        {rendering ? "Rendering…" : "Render PDF"}
      </button>
    </div>
  </div>
</section>
