// Render the invoice example with fetch. No dependencies; Node 20+.
// Usage: TYPREST_URL=http://localhost:8080 node --experimental-strip-types client.ts [payload.json] [output.pdf]
import { readFile, writeFile } from "node:fs/promises";

interface Violation {
  field: string;
  issue: string;
}
interface Diagnostic {
  severity: string;
  message: string;
  location?: { path: string; range?: { start: { line: number; column: number } } };
}
interface ErrorEnvelope {
  error: string;
  message: string;
  violations?: Violation[];
  diagnostics?: Diagnostic[];
  line?: number;
}

const baseUrl = (process.env.TYPREST_URL ?? "http://localhost:8080").replace(/\/+$/, "");
const payloadPath =
  process.argv[2] ?? new URL("../invoice/templates/invoice/payload.json", import.meta.url).pathname;
const outputPath = process.argv[3] ?? "invoice.pdf";

async function main(): Promise<number> {
  const payload = await readFile(payloadPath, "utf8");
  const response = await fetch(`${baseUrl}/invoice`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: payload,
  });

  if (response.ok) {
    const pdf = Buffer.from(await response.arrayBuffer());
    await writeFile(outputPath, pdf);
    console.log(`saved ${outputPath} (${pdf.length} bytes)`);
    return 0;
  }

  console.error(`request failed: HTTP ${response.status}`);
  const text = await response.text();
  try {
    const envelope = JSON.parse(text) as ErrorEnvelope;
    console.error(`${envelope.error}: ${envelope.message}`);
    for (const v of envelope.violations ?? []) console.error(`  ${v.field}: ${v.issue}`);
    for (const d of envelope.diagnostics ?? []) {
      const start = d.location?.range?.start;
      const where = d.location ? ` (${d.location.path}${start ? `:${start.line}` : ""})` : "";
      console.error(`  [${d.severity}] ${d.message}${where}`);
    }
  } catch {
    console.error(text);
  }
  return 1;
}

main().then(
  (code) => process.exit(code),
  (err) => {
    console.error(`request error: ${err instanceof Error ? err.message : err}`);
    process.exit(1);
  },
);
