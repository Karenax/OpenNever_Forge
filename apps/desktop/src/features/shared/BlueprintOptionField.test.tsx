import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EditableBlueprintOptionField, formatBlueprintOption } from "./BlueprintOptionField";

const options = [
  { value: 0, label: "Masculin", source: "règle Aurora Gender" },
  { value: 1, label: "Féminin", source: "règle Aurora Gender" },
];

describe("BlueprintOptionField", () => {
  it("shows the resolved symbol while retaining the Aurora value", () => {
    expect(formatBlueprintOption({ kind: "byte", value: 1 }, options)).toBe("Féminin · 1");
  });

  it("commits the selected symbolic option with the original GFF kind", async () => {
    const onCommit = vi.fn().mockResolvedValue(undefined);
    render(<EditableBlueprintOptionField label="Genre" value={{ kind: "byte", value: 0 }} options={options} onCommit={onCommit} />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "1" } });
    expect(screen.getByText("règle Aurora Gender")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Appliquer" }));
    await waitFor(() => expect(onCommit).toHaveBeenCalledWith({ kind: "byte", value: 1 }));
  });
});
