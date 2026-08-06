import { beforeEach, describe, expect, it } from "vitest";
import {
  EMPTY_PROJECT_PREFERENCES,
  PROJECT_PREFERENCES_STORAGE_KEY,
  loadProjectPreferences,
  saveProjectPreferences,
} from "./projectPreferences";

describe("project preferences", () => {
  beforeEach(() => localStorage.clear());

  it("round-trips the three project paths", () => {
    const project = {
      modulePath: "E:/Modules/campaign.mod",
      gameInstallPath: "E:/Games/Neverwinter Nights",
      userDataPath: "E:/Documents/Neverwinter Nights",
    };

    saveProjectPreferences(project);

    expect(loadProjectPreferences()).toEqual(project);
  });

  it("ignores corrupt or incompatible persisted data", () => {
    localStorage.setItem(PROJECT_PREFERENCES_STORAGE_KEY, "not-json");
    expect(loadProjectPreferences()).toEqual(EMPTY_PROJECT_PREFERENCES);

    localStorage.setItem(
      PROJECT_PREFERENCES_STORAGE_KEY,
      JSON.stringify({ version: 2, project: { modulePath: "unsafe" } }),
    );
    expect(loadProjectPreferences()).toEqual(EMPTY_PROJECT_PREFERENCES);
  });
});
