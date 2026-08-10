import { useState } from "react";
import type { BlueprintFieldOption, GenericGffValue } from "../lib/tauri";

export function formatBlueprintOption(value: GenericGffValue, options?: BlueprintFieldOption[]) {
  const raw = typeof value.value === "number" ? value.value : Number(value.value);
  const option = Number.isFinite(raw) ? options?.find((candidate) => candidate.value === raw) : undefined;
  return option ? `${option.label} · ${raw}` : String(value.value ?? "");
}

export function EditableBlueprintOptionField({
  label,
  value,
  options,
  onCommit,
}: {
  label: string;
  value: GenericGffValue;
  options: BlueprintFieldOption[];
  onCommit: (after: GenericGffValue) => Promise<void>;
}) {
  const original = Number(value.value);
  const [draft, setDraft] = useState(original);
  const [busy, setBusy] = useState(false);
  const commit = async () => {
    setBusy(true);
    try {
      await onCommit({ kind: value.kind, value: draft });
    } finally {
      setBusy(false);
    }
  };
  return (
    <label className="gff-field-row">
      <span>{label}</span>
      <select value={draft} onChange={(event) => setDraft(Number(event.currentTarget.value))}>
        {!options.some((option) => option.value === original) && <option value={original}>Valeur actuelle · {original}</option>}
        {options.map((option) => <option key={option.value} value={option.value}>{option.label} · {option.value}</option>)}
      </select>
      <small>{options.find((option) => option.value === draft)?.source ?? "Valeur non résolue par les ressources actives"}</small>
      <button type="button" disabled={busy || draft === original} onClick={() => void commit()}>{busy ? "…" : "Appliquer"}</button>
    </label>
  );
}
