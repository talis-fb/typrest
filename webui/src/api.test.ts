import { afterEach, describe, expect, it, vi } from "vitest";

import {
  encodeTemplateId,
  getTemplate,
  renderTemplate,
  templatePdfFilename,
} from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("template ID URLs", () => {
  it("encodes individual segments without hiding separators", () => {
    expect(encodeTemplateId("financial/invoice_v2")).toBe(
      "financial/invoice_v2",
    );
  });

  it("uses nested IDs for metadata and direct rendering", async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ id: "financial/invoice", schema: null }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(new Blob(["%PDF"]), {
          status: 200,
          headers: { "Content-Type": "application/pdf" },
        }),
      );
    vi.stubGlobal("fetch", fetch);

    await getTemplate("financial/invoice");
    await renderTemplate("financial/invoice", {});

    expect(fetch.mock.calls[0]?.[0]).toBe(
      "/_api/templates/financial/invoice",
    );
    expect(fetch.mock.calls[1]?.[0]).toBe("/financial/invoice");
    expect(fetch.mock.calls[1]?.[1]).toMatchObject({ method: "POST" });
  });

  it("creates a flat download filename", () => {
    expect(templatePdfFilename("financial/invoice")).toBe(
      "financial--invoice.pdf",
    );
  });
});
