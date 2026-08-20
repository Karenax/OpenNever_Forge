import { gzipSync } from "node:zlib";
import { readFileSync, readdirSync } from "node:fs";
import { basename, join, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dirname, "..");
const distRoot = join(repositoryRoot, "apps", "desktop", "dist");
const indexHtml = readFileSync(join(distRoot, "index.html"), "utf8");
const entryMatch = indexHtml.match(/<script[^>]+src="[./]*assets\/([^"]+\.js)"/);
if (!entryMatch) throw new Error("Le bundle ne contient aucun script d'entrée identifiable.");

const budgets = {
  entryBytes: 3_300_000,
  entryGzipBytes: 900_000,
  largestChunkBytes: 4_000_000,
  largestChunkGzipBytes: 950_000,
  cssBytes: 230_000,
};
const assetRoot = join(distRoot, "assets");
const files = readdirSync(assetRoot)
  .filter((name) => name.endsWith(".js") || name.endsWith(".css"))
  .map((name) => {
    const bytes = readFileSync(join(assetRoot, name));
    return { name, bytes: bytes.byteLength, gzipBytes: gzipSync(bytes).byteLength };
  });
const entry = files.find((file) => file.name === basename(entryMatch[1]));
if (!entry) throw new Error(`Chunk d'entrée absent : ${entryMatch[1]}`);
const largestChunk = files.filter((file) => file.name.endsWith(".js")).sort((a, b) => b.bytes - a.bytes)[0];
const largestCss = files.filter((file) => file.name.endsWith(".css")).sort((a, b) => b.bytes - a.bytes)[0];

// These are containment ceilings for existing monoliths, not growth targets.
// New work should extract modules instead of routinely raising these limits.
const sourceBudgets = [
  ["apps/desktop/src/App.tsx", 3_050],
  ["apps/desktop/src-tauri/src/commands.rs", 7_700],
  ["crates/aurora-edit/src/lib.rs", 9_950],
];
const sources = sourceBudgets.map(([relativePath, maximumLines]) => {
  const lines = readFileSync(join(repositoryRoot, relativePath), "utf8").split(/\r?\n/).length;
  return { relativePath, lines, maximumLines };
});
const failures = [];
if (entry.bytes > budgets.entryBytes) failures.push(`entrée ${entry.bytes} > ${budgets.entryBytes}`);
if (entry.gzipBytes > budgets.entryGzipBytes) failures.push(`entrée gzip ${entry.gzipBytes} > ${budgets.entryGzipBytes}`);
if (largestChunk.bytes > budgets.largestChunkBytes) failures.push(`chunk ${largestChunk.bytes} > ${budgets.largestChunkBytes}`);
if (largestChunk.gzipBytes > budgets.largestChunkGzipBytes) failures.push(`chunk gzip ${largestChunk.gzipBytes} > ${budgets.largestChunkGzipBytes}`);
if (largestCss?.bytes > budgets.cssBytes) failures.push(`CSS ${largestCss.bytes} > ${budgets.cssBytes}`);
for (const source of sources) {
  if (source.lines > source.maximumLines) failures.push(`${source.relativePath} ${source.lines} lignes > ${source.maximumLines}`);
}

console.log(JSON.stringify({ entry, largestChunk, largestCss, sources, budgets }, null, 2));
if (failures.length) {
  throw new Error(`Budgets dépassés : ${failures.join(" ; ")}`);
}
