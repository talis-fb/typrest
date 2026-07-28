export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

export type SchemaType =
  | "string"
  | "number"
  | "boolean"
  | "object"
  | "array"
  | "null";

export interface SchemaField {
  type?: SchemaType;
  required?: boolean;
  properties?: Record<string, SchemaField>;
  items?: SchemaField;
  enum?: JsonValue[];
  default?: JsonValue;
}

export interface Schema {
  schemaVersion: string;
  request: {
    body: SchemaField;
  };
}

export interface TemplateSummary {
  name: string;
  hasSchema: boolean;
}

export interface TemplateMetadata {
  name: string;
  schema: Schema | null;
}

export interface ApiErrorBody {
  error?: string;
  message?: string;
  violations?: Array<{ field: string; issue: string }>;
  diagnostics?: Array<{
    severity: string;
    message: string;
    location?: {
      path: string;
      range: {
        start: { line: number; column: number };
        end: { line: number; column: number };
      };
    };
    hints?: Array<{ message: string }>;
    trace?: Array<{ message: string }>;
  }>;
}

export class ApiRequestError extends Error {
  constructor(
    message: string,
    public readonly status: number | null,
    public readonly body: ApiErrorBody | null,
  ) {
    super(message);
    this.name = "ApiRequestError";
  }
}

export async function listTemplates(signal?: AbortSignal): Promise<TemplateSummary[]> {
  const response = await fetch("/_api/templates", { signal });
  if (!response.ok) {
    throw await requestError(response, "Could not load templates");
  }
  const body = (await response.json()) as { templates: TemplateSummary[] };
  return body.templates;
}

export async function getTemplate(
  name: string,
  signal?: AbortSignal,
): Promise<TemplateMetadata> {
  const response = await fetch(`/_api/templates/${encodeURIComponent(name)}`, { signal });
  if (!response.ok) {
    throw await requestError(response, `Could not load template "${name}"`);
  }
  return (await response.json()) as TemplateMetadata;
}

export async function renderTemplate(
  name: string,
  payload: JsonValue,
  signal?: AbortSignal,
): Promise<Blob> {
  let response: Response;
  try {
    response = await fetch(`/render/${encodeURIComponent(name)}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      signal,
    });
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw error;
    }
    throw new ApiRequestError("Network error while rendering the PDF", null, null);
  }

  if (!response.ok) {
    throw await requestError(response, "PDF rendering failed");
  }

  return response.blob();
}

async function requestError(response: Response, fallback: string): Promise<ApiRequestError> {
  let body: ApiErrorBody | null = null;
  try {
    body = (await response.json()) as ApiErrorBody;
  } catch {
    // Non-JSON upstream errors still receive a useful status-aware fallback.
  }
  return new ApiRequestError(body?.message ?? fallback, response.status, body);
}
