import { readFile } from "node:fs/promises";
import process from "node:process";
import validator from "gltf-validator";

const path = process.argv[2];
if (!path) {
  console.error("usage: pnpm validate:glb <asset.glb>");
  process.exitCode = 2;
} else {
  const bytes = await readFile(path);
  const report = await validator.validateBytes(new Uint8Array(bytes), {
    externalResourceFunction: async () => {
      throw new Error("OpenNever Forge GLB previews must be self-contained");
    },
  });
  const summary = {
    uri: path,
    validatorVersion: report.validatorVersion,
    errors: report.issues.numErrors,
    warnings: report.issues.numWarnings,
    infos: report.issues.numInfos,
    hints: report.issues.numHints,
    messages: report.issues.messages,
  };
  console.log(JSON.stringify(summary, null, 2));
  if (report.issues.numErrors > 0) {
    process.exitCode = 1;
  }
}
