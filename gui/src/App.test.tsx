import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import App from "./App";

// Mock the API module
vi.mock("./api", () => ({
  invokeSearch: vi.fn(),
  fetchSites: vi.fn().mockResolvedValue(["fitgirl", "dodi", "gog-games"]),
  // Cache API mocks
  getCache: vi.fn().mockResolvedValue([]),
  getCachedResults: vi.fn().mockResolvedValue(null),
  addToCache: vi.fn().mockResolvedValue(undefined),
  removeCacheEntry: vi.fn().mockResolvedValue(true),
  clearCache: vi.fn().mockResolvedValue(undefined),
  getCacheSettings: vi.fn().mockResolvedValue(100),
  setCacheSize: vi.fn().mockResolvedValue(undefined),
}));

import { invokeSearch, fetchSites } from "./api";
const mockedInvokeSearch = vi.mocked(invokeSearch);
const mockedFetchSites = vi.mocked(fetchSites);

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedFetchSites.mockResolvedValue(["fitgirl", "dodi", "gog-games"]);
  });

  it("renders search input and button", () => {
    render(<App />);
    expect(screen.getByPlaceholderText("e.g., elden ring")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /search/i })).toBeInTheDocument();
  });

  it("renders heading", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: "Website Searcher" })
    ).toBeInTheDocument();
  });

  it("shows error on empty search", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /search/i }));
    await waitFor(() => {
      expect(screen.getByText("Enter a search phrase")).toBeInTheDocument();
    });
  });

  it("displays site checkboxes after loading", async () => {
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("fitgirl")).toBeInTheDocument();
    });
    expect(screen.getByText("dodi")).toBeInTheDocument();
    expect(screen.getByText("gog-games")).toBeInTheDocument();
  });

  it("allows typing in search input", () => {
    render(<App />);
    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "test query" } });
    expect(input).toHaveValue("test query");
  });

  it("calls invokeSearch when search button clicked with query", async () => {
    mockedInvokeSearch.mockResolvedValue([]);
    render(<App />);

    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "elden ring" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(mockedInvokeSearch).toHaveBeenCalledWith(
        expect.objectContaining({ query: "elden ring" })
      );
    });
  });

  it("displays results grouped by site", async () => {
    mockedInvokeSearch.mockResolvedValue([
      { site: "fitgirl", title: "Game 1", url: "http://example.com/1" },
      { site: "fitgirl", title: "Game 2", url: "http://example.com/2" },
      { site: "dodi", title: "Game 3", url: "http://example.com/3" },
    ]);

    render(<App />);
    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "game" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      // Should show result cards with site headers (h3 elements)
      const headings = screen.getAllByRole("heading", { level: 3 });
      const headingTexts = headings.map((h) => h.textContent);
      expect(headingTexts).toContain("fitgirl");
      expect(headingTexts).toContain("dodi");
    });
  });

  it('shows "No results yet" when no results and not loading', () => {
    render(<App />);
    expect(screen.getByText("No results yet.")).toBeInTheDocument();
  });

  it("shows loading state when searching", async () => {
    // Make the mock hang indefinitely
    mockedInvokeSearch.mockImplementation(() => new Promise(() => {}));

    render(<App />);
    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(screen.getByText("Searching…")).toBeInTheDocument();
    });
  });

  it("shows error message on search failure", async () => {
    mockedInvokeSearch.mockRejectedValue(new Error("Network error"));

    render(<App />);
    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(screen.getByText("Network error")).toBeInTheDocument();
    });
  });

  it("allows selecting sites via checkboxes", async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("fitgirl")).toBeInTheDocument();
    });

    // Find checkbox by its associated label text
    const checkboxes = screen.getAllByRole("checkbox");
    // The first few checkboxes are for site selection
    const fitgirlCheckbox = checkboxes[0];
    fireEvent.click(fitgirlCheckbox);
    expect(fitgirlCheckbox).toBeChecked();
  });

  it("copies URL to clipboard when link clicked", async () => {
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: { writeText: writeTextMock },
    });

    mockedInvokeSearch.mockResolvedValue([
      { site: "fitgirl", title: "Game", url: "http://example.com/game" },
    ]);

    render(<App />);
    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "game" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(screen.getByText("http://example.com/game")).toBeInTheDocument();
    });

    const link = screen.getByText("http://example.com/game");
    fireEvent.click(link);

    await waitFor(() => {
      expect(writeTextMock).toHaveBeenCalledWith("http://example.com/game");
    });
  });

  it("shows copied toast after copying", async () => {
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: { writeText: writeTextMock },
    });

    mockedInvokeSearch.mockResolvedValue([
      { site: "fitgirl", title: "Game", url: "http://example.com/game" },
    ]);

    render(<App />);
    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "game" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(screen.getByText("http://example.com/game")).toBeInTheDocument();
    });

    const link = screen.getByText("http://example.com/game");
    fireEvent.click(link);

    await waitFor(() => {
      expect(screen.getByText("Copied!")).toBeInTheDocument();
    });
  });

  it("allows changing limit input", () => {
    render(<App />);
    const limitInput = screen.getByDisplayValue("10");
    fireEvent.change(limitInput, { target: { value: "25" } });
    expect(limitInput).toHaveValue(25);
  });

  it("allows typing CF URL", () => {
    render(<App />);
    const cfInput = screen.getByPlaceholderText("http://localhost:8191/v1");
    fireEvent.change(cfInput, { target: { value: "http://mycf:8191/v1" } });
    expect(cfInput).toHaveValue("http://mycf:8191/v1");
  });

  it("allows typing Cookie", () => {
    render(<App />);
    const cookieInput = screen.getByPlaceholderText("key=value; other=value2");
    fireEvent.change(cookieInput, { target: { value: "session=abc123" } });
    expect(cookieInput).toHaveValue("session=abc123");
  });

  it("allows changing csrin_pages input", () => {
    render(<App />);
    // csrin_pages defaults to 1
    const csrinPagesInput = screen.getByDisplayValue("1");
    fireEvent.change(csrinPagesInput, { target: { value: "3" } });
    expect(csrinPagesInput).toHaveValue(3);
  });

  it("toggles csrin_search checkbox", () => {
    render(<App />);
    const checkbox = screen.getByRole("checkbox", { name: /csrin_search/i });
    expect(checkbox).not.toBeChecked();
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it("toggles no_playwright checkbox", () => {
    render(<App />);
    const checkbox = screen.getByRole("checkbox", { name: /no_playwright/i });
    expect(checkbox).not.toBeChecked();
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it("toggles no_cf checkbox", () => {
    render(<App />);
    const checkbox = screen.getByRole("checkbox", { name: /no_cf/i });
    expect(checkbox).not.toBeChecked();
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it("toggles debug checkbox", () => {
    render(<App />);
    const checkbox = screen.getByRole("checkbox", { name: /debug/i });
    expect(checkbox).not.toBeChecked();
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it("handles fetchSites error gracefully", async () => {
    mockedFetchSites.mockRejectedValue(new Error("Network error"));
    render(<App />);
    // Should still render without crashing, showing loading state initially
    await waitFor(() => {
      // After error, sites array is empty but app doesn't crash
      expect(
        screen.getByRole("heading", { name: "Website Searcher" })
      ).toBeInTheDocument();
    });
  });

  it("passes selected sites to invokeSearch", async () => {
    mockedInvokeSearch.mockResolvedValue([]);
    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("fitgirl")).toBeInTheDocument();
    });

    // Select fitgirl site
    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]); // fitgirl checkbox

    const input = screen.getByPlaceholderText("e.g., elden ring");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(mockedInvokeSearch).toHaveBeenCalledWith(
        expect.objectContaining({
          query: "test",
          sites: ["fitgirl"],
        })
      );
    });
  });
});

