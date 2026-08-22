import { open, save } from "@tauri-apps/plugin-dialog";
import { requireTauri } from "./errors";

export async function selectModuleOutput(defaultPath = "opennever-build.mod"): Promise<string | null> {
  requireTauri();
  return save({ defaultPath, filters: [{ name: "Module Neverwinter Nights", extensions: ["mod"] }] });
}

export async function selectHakOutput(defaultPath = "opennever-content.hak"): Promise<string | null> {
  requireTauri();
  return save({ defaultPath, filters: [{ name: "Hakpak Neverwinter Nights", extensions: ["hak"] }] });
}

export async function selectModule(): Promise<string | null> {
  requireTauri();
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Module ou carte Neverwinter Nights", extensions: ["mod", "are"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function selectDirectory(): Promise<string | null> {
  requireTauri();
  const selected = await open({ multiple: false, directory: true });
  return typeof selected === "string" ? selected : null;
}

export async function selectCompiler(): Promise<string | null> {
  requireTauri();
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Compilateur NWScript", extensions: ["exe"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function selectNwnExecutable(): Promise<string | null> {
  requireTauri();
  const selected = await open({ multiple: false, directory: false, filters: [{ name: "Neverwinter Nights", extensions: ["exe"] }] });
  return typeof selected === "string" ? selected : null;
}
