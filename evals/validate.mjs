import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const suites = [
  { name: "persona", path: "evals/cases/persona/golden.jsonl", minimum: 20 },
  { name: "memory", path: "evals/cases/memory/golden.jsonl", minimum: 20 },
];

const ids = new Set();
const secretPattern = /(?:sk-[A-Za-z0-9]{16,}|Bearer\s+[A-Za-z0-9._-]{16,}|BEGIN (?:RSA |EC )?PRIVATE KEY)/;

function invariant(condition, message) {
  if (!condition) throw new Error(message);
}

for (const suite of suites) {
  const file = resolve(suite.path);
  const text = await readFile(file, "utf8");
  const lines = text.split(/\r?\n/).filter((line) => line.trim().length > 0);
  invariant(lines.length >= suite.minimum, `${suite.name}: expected at least ${suite.minimum} cases`);

  const categories = new Set();
  for (const [index, line] of lines.entries()) {
    let item;
    try {
      item = JSON.parse(line);
    } catch (error) {
      throw new Error(`${suite.name}:${index + 1}: invalid JSON: ${error.message}`);
    }

    invariant(typeof item.id === "string" && item.id.startsWith(`${suite.name}-`), `${suite.name}:${index + 1}: invalid id`);
    invariant(!ids.has(item.id), `${suite.name}:${index + 1}: duplicate id ${item.id}`);
    ids.add(item.id);
    invariant(typeof item.category === "string" && item.category.length > 0, `${item.id}: missing category`);
    invariant(item.input && typeof item.input === "object", `${item.id}: missing input`);
    invariant(item.expected && typeof item.expected === "object", `${item.id}: missing expected`);
    invariant(Array.isArray(item.tags) && item.tags.length > 0, `${item.id}: missing tags`);
    invariant(!secretPattern.test(line), `${item.id}: looks like a real secret`);
    categories.add(item.category);

    if (suite.name === "persona") {
      invariant(Array.isArray(item.input.messages) && item.input.messages.length > 0, `${item.id}: missing messages`);
      invariant(Array.isArray(item.expected.must) && Array.isArray(item.expected.must_not), `${item.id}: missing persona constraints`);
    } else {
      invariant(Array.isArray(item.input.records), `${item.id}: missing records`);
      invariant(typeof item.input.query === "string", `${item.id}: missing query`);
      invariant(Array.isArray(item.expected.recall_ids) && Array.isArray(item.expected.exclude_ids), `${item.id}: missing recall constraints`);
    }
  }

  invariant(categories.size >= 5, `${suite.name}: expected at least 5 categories`);
  console.log(`${suite.name}: ${lines.length} cases, ${categories.size} categories`);
}

const personaBaselines = [
  "evals/results/p0/persona.jsonl",
  "evals/results/p0/persona-candidate.jsonl",
];
for (const baseline of personaBaselines) {
  const lines = (await readFile(resolve(baseline), "utf8")).split(/\r?\n/).filter(Boolean);
  invariant(lines.length === 20, `${baseline}: expected exactly 20 records`);
  const baselineIds = new Set();
  for (const [index, line] of lines.entries()) {
    const record = JSON.parse(line);
    invariant(typeof record.id === "string", `${baseline}:${index + 1}: missing id`);
    invariant(!baselineIds.has(record.id), `${baseline}: duplicate id ${record.id}`);
    baselineIds.add(record.id);
    invariant(typeof record.promptSha256 === "string" && record.promptSha256.length === 64, `${record.id}: missing prompt hash`);
    invariant(typeof record.generation?.content === "string", `${record.id}: missing generation`);
    invariant(record.systemPrompt === undefined, `${record.id}: full system prompt must not be stored`);
    invariant(!secretPattern.test(line), `${record.id}: result looks like a real secret`);
  }
}

const manualReview = JSON.parse(await readFile(resolve("evals/results/p0/persona-candidate-review.json"), "utf8"));
invariant(manualReview.results?.length === 20, "candidate review: expected 20 results");
invariant(manualReview.summary?.passed + manualReview.summary?.failed === 20, "candidate review: invalid summary");

const harness = JSON.parse(await readFile(resolve("evals/results/p0/harness.json"), "utf8"));
invariant(harness.callsUsed === 5 && harness.runs?.length === 5, "harness: expected 5 runs");
invariant(harness.storage?.iterations === 200, "harness: expected 200 storage iterations");

console.log(`eval fixtures valid: ${ids.size} cases plus online P0 baselines`);
