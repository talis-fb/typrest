<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { JsonValue, SchemaField } from "../api";
  import { isJsonObject, seedField } from "../payload";

  export let name: string;
  export let path: string;
  export let field: SchemaField;
  export let value: JsonValue | undefined;
  export let included = false;
  export let onValidity: (path: string, issue: string | null) => void = () => {};

  const dispatch = createEventDispatcher<{
    change: { included: boolean; value: JsonValue | undefined };
  }>();

  let jsonDraft = JSON.stringify(value ?? seedField(field), null, 2);
  let jsonError = "";
  let scalarError = "";

  $: if (!included) {
    onValidity(path, null);
  }

  function toggleIncluded(next: boolean) {
    included = next;
    const nextValue = next ? (value ?? seedField(field)) : undefined;
    if (nextValue !== undefined) {
      jsonDraft = JSON.stringify(nextValue, null, 2);
    }
    scalarError = "";
    dispatch("change", { included: next, value: nextValue });
  }

  function change(nextValue: JsonValue) {
    dispatch("change", { included: true, value: nextValue });
  }

  function updateNumber(nextValue: number) {
    if (Number.isNaN(nextValue)) {
      scalarError = "Enter a valid number";
      onValidity(path, "Enter a valid number");
      return;
    }
    scalarError = "";
    onValidity(path, null);
    change(nextValue);
  }

  function changeObjectProperty(
    propertyName: string,
    detail: { included: boolean; value: JsonValue | undefined },
  ) {
    const object = isJsonObject(value ?? null) ? { ...(value as Record<string, JsonValue>) } : {};
    if (detail.included && detail.value !== undefined) {
      object[propertyName] = detail.value;
    } else {
      delete object[propertyName];
    }
    change(object);
  }

  function updateJson(text: string) {
    jsonDraft = text;
    try {
      const parsed = JSON.parse(text) as JsonValue;
      if (field.type === "array" && !Array.isArray(parsed)) {
        throw new Error("Expected a JSON array");
      }
      jsonError = "";
      onValidity(path, null);
      change(parsed);
    } catch (error) {
      jsonError = error instanceof Error ? error.message : "Invalid JSON value";
      onValidity(path, jsonError);
    }
  }

  function enumValue(text: string) {
    change(JSON.parse(text) as JsonValue);
  }
</script>

<fieldset class="rounded-xl border border-base-300 bg-base-100 p-3">
  <legend class="px-1 text-sm font-semibold">
    {name}
    {#if field.required}
      <span class="text-error" aria-label="required">*</span>
    {/if}
  </legend>

  {#if !field.required}
    <label class="mb-3 flex cursor-pointer items-center gap-2 text-xs text-base-content/65">
      <input
        class="checkbox checkbox-primary checkbox-xs"
        type="checkbox"
        checked={included}
        on:change={(event) => toggleIncluded(event.currentTarget.checked)}
      />
      Include in payload
    </label>
  {/if}

  {#if included || field.required}
    {#if field.enum && field.enum.length > 0}
      <label class="form-control">
        <span class="label-text sr-only">{name}</span>
        <select
          class="select select-bordered w-full"
          value={JSON.stringify(value ?? field.enum[0])}
          on:change={(event) => enumValue(event.currentTarget.value)}
        >
          {#each field.enum as option}
            <option value={JSON.stringify(option)}>{String(option)}</option>
          {/each}
        </select>
      </label>
    {:else if field.type === "string"}
      <input
        class:input-error={scalarError}
        class="input input-bordered w-full"
        aria-label={name}
        type="text"
        value={typeof value === "string" ? value : ""}
        on:input={(event) => change(event.currentTarget.value)}
      />
    {:else if field.type === "number"}
      <input
        class="input input-bordered w-full"
        aria-label={name}
        type="number"
        step="any"
        value={typeof value === "number" ? value : 0}
        on:input={(event) => updateNumber(event.currentTarget.valueAsNumber)}
      />
      {#if scalarError}
        <p class="mt-1 text-xs text-error" role="alert">{scalarError}</p>
      {/if}
    {:else if field.type === "boolean"}
      <label class="flex cursor-pointer items-center gap-3 rounded-lg bg-base-200 px-3 py-2">
        <input
          class="toggle toggle-primary"
          type="checkbox"
          checked={value === true}
          on:change={(event) => change(event.currentTarget.checked)}
        />
        <span class="text-sm">{value === true ? "true" : "false"}</span>
      </label>
    {:else if field.type === "null"}
      <div class="rounded-lg bg-base-200 px-3 py-2 font-mono text-sm text-base-content/60">null</div>
    {:else if field.type === "object"}
      <div class="space-y-3">
        {#each Object.entries(field.properties ?? {}) as [propertyName, property]}
          {@const objectValue = isJsonObject(value ?? null) ? value as Record<string, JsonValue> : {}}
          <svelte:self
            name={propertyName}
            path={`${path}.${propertyName}`}
            field={property}
            value={objectValue[propertyName]}
            included={property.required || Object.prototype.hasOwnProperty.call(objectValue, propertyName)}
            {onValidity}
            on:change={(event) => changeObjectProperty(propertyName, event.detail)}
          />
        {/each}
        {#if Object.keys(field.properties ?? {}).length === 0}
          <p class="text-sm text-base-content/55">This object has no declared properties.</p>
        {/if}
      </div>
    {:else}
      <label class="form-control">
        <span class="label-text mb-1 text-xs text-base-content/60">
          {field.type === "array" ? "JSON array" : "JSON value"}
        </span>
        <textarea
          class:textarea-error={jsonError}
          class="textarea textarea-bordered min-h-24 w-full font-mono text-sm"
          aria-label={`${name} JSON value`}
          value={jsonDraft}
          on:input={(event) => updateJson(event.currentTarget.value)}
        ></textarea>
      </label>
      {#if jsonError}
        <p class="mt-1 text-xs text-error" role="alert">{jsonError}</p>
      {/if}
    {/if}
  {/if}
</fieldset>
