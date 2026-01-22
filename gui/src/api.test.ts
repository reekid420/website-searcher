import { describe, it, expect, vi, beforeEach } from "vitest";
import { invokeSearch, fetchSites } from "./api";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockedInvoke = vi.mocked(invoke);

describe("api", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("invokeSearch", () => {
    it("throws on empty query", async () => {
      await expect(invokeSearch({ query: "" })).rejects.toThrow(
        "Query is required"
      );
    });

    it("throws on whitespace-only query", async () => {
      await expect(invokeSearch({ query: "   " })).rejects.toThrow(
        "Query is required"
      );
    });

    it("calls invoke with correct command name", async () => {
      mockedInvoke.mockResolvedValue([]);
      await invokeSearch({ query: "elden ring" });
      expect(mockedInvoke).toHaveBeenCalledWith("search_gui", {
        args: { query: "elden ring" },
      });
    });

    it("passes all options to invoke", async () => {
      mockedInvoke.mockResolvedValue([]);
      await invokeSearch({
        query: "test",
        limit: 5,
        sites: ["fitgirl", "dodi"],
        debug: true,
        no_cf: true,
      });
      expect(mockedInvoke).toHaveBeenCalledWith("search_gui", {
        args: {
          query: "test",
          limit: 5,
          sites: ["fitgirl", "dodi"],
          debug: true,
          no_cf: true,
        },
      });
    });

    it("returns search results from invoke", async () => {
      const mockResults = [
        { site: "fitgirl", title: "Game", url: "http://example.com" },
      ];
      mockedInvoke.mockResolvedValue(mockResults);
      const result = await invokeSearch({ query: "game" });
      expect(result).toEqual(mockResults);
    });
  });

  describe("fetchSites", () => {
    it("calls invoke with list_sites command", async () => {
      mockedInvoke.mockResolvedValue(["fitgirl", "dodi"]);
      await fetchSites();
      expect(mockedInvoke).toHaveBeenCalledWith("list_sites");
    });

    it("returns site list from invoke", async () => {
      const mockSites = ["fitgirl", "dodi", "gog-games"];
      mockedInvoke.mockResolvedValue(mockSites);
      const result = await fetchSites();
      expect(result).toEqual(mockSites);
    });
  });
});
