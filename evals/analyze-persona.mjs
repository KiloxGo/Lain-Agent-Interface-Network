import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const path = resolve(process.env.PERSONA_BASELINE_PATH ?? "evals/results/p0/persona.jsonl");
const records = (await readFile(path, "utf8"))
  .split(/\r?\n/)
  .filter(Boolean)
  .map((line) => JSON.parse(line));

if (records.some((record) => !record.judgment)) {
  console.log("manual review cases");
  for (const record of records) {
    console.log(`\n${record.id}\t${record.category}`);
    console.log(`must: ${record.expected.must.join(" | ")}`);
    console.log(`must_not: ${record.expected.must_not.join(" | ")}`);
    console.log(`response: ${record.generation.content.replace(/\s+/g, " ")}`);
  }
  process.exit(0);
}

const categories = new Map();
for (const record of records) {
  const aggregate = categories.get(record.category) ?? { total: 0, passed: 0 };
  aggregate.total += 1;
  if (record.judgment.overall_pass) aggregate.passed += 1;
  categories.set(record.category, aggregate);
}

console.log("category\tpassed\ttotal");
for (const [category, aggregate] of [...categories.entries()].sort()) {
  console.log(`${category}\t${aggregate.passed}\t${aggregate.total}`);
}

console.log("\nfailed cases");
for (const record of records.filter((item) => !item.judgment.overall_pass)) {
  const response = record.generation.content.replace(/\s+/g, " ").slice(0, 120);
  console.log(`${record.id}\t${record.category}\t${record.judgment.reason}\t${response}`);
}
