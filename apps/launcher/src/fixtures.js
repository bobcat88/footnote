// Invented sample data, shaped like the engine's real JSON, for browser screenshots only (`?mock=1`). Never shown in the app.
const models = {
  folder: "C:/Users/sample/AppData/Local/footnote/models",
  memory_uses_from_elsewhere: [],
  models: [
    { name: "bekko-a8m", installed: true, bytes: 164462521, license: "MIT", role: "default pair, multilingual half", used_by_memory: true },
    { name: "gte-small", installed: true, bytes: 34726087, license: "MIT", role: "default pair, English half", used_by_memory: true },
    { name: "e5-small", installed: false, download_bytes: 135390915, license: "MIT", role: "earlier model, tests and comparison", used_by_memory: false },
    { name: "jina-v5-nano", installed: false, download_bytes: 232883776, license: "CC-BY-NC-4.0 (no commercial use)", role: "French or mixed corpora, opt-in (llama build)", used_by_memory: false },
  ],
};

const status = {
  version: "0.1.0-alpha.1",
  models,
  products: {
    code: { built: true, enabled: true, graphs: 2, rust_indexer: true },
    library: { built: true, enabled: true },
    memory: {
      built: true, enabled: true, memories: 318, saved_last_7_days: 41, recalls_last_7_days: 276, daemon_running: true,
      agents: [
        { agent: "claude-code", hooks: true, mcp_server: true },
        { agent: "codex", hooks: true, mcp_server: true },
        { agent: "deepseek-harness", hooks: false, mcp_server: true },
      ],
    },
  },
};

const today = new Date();
const iso = (back) => {
  const d = new Date(today);
  d.setDate(today.getDate() - back);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};

const projects = {
  projects: [
    { name: "atlas-api", path: "D:/work/atlas-api", exists: true, first_seen: iso(20), last_seen: iso(0), memories: 96, code_graph: true, git: { uncommitted: 0, unpushed: 0 } },
    { name: "harbor-web", path: "D:/work/harbor-web", exists: true, first_seen: iso(12), last_seen: iso(1), memories: 31, code_graph: false, git: { uncommitted: 12, unpushed: 3 } },
    { name: "ledger-sync", path: "D:/work/ledger-sync", exists: true, first_seen: iso(9), last_seen: iso(3), memories: 18, code_graph: true, git: { uncommitted: 0, unpushed: 5 } },
    { name: "old-prototype", path: "D:/archive/old-prototype", exists: false, first_seen: iso(30), last_seen: iso(14), memories: 4, code_graph: false, git: null },
  ],
};

const activity = {
  activity_log: true,
  note: "dates in UTC; saves by the date the agent observed; prompt text is never recorded",
  days: [6, 5, 4, 3, 2, 1, 0].map((back, i) => ({
    date: iso(back),
    saved: [0, 3, 7, 0, 12, 9, 10][i],
    prompts_given_memories: [0, 22, 41, 0, 63, 70, 80][i],
    sessions: [0, 2, 4, 0, 6, 5, 7][i],
  })),
  recent: [
    { t: Date.UTC(2026, 10, 17, 9, 12), event: "recall", project: "atlas-api", memories: [
      { id: 17, kind: "fact", scope: "atlas-api", observed_at: "2026-11-12", text: "The API keeps cursor pagination; offset pagination was dropped after the slow-count incident." },
      { id: 42, kind: "preference", scope: "global", observed_at: "2026-11-10", text: "Prefers small pull requests, one per piece of work, with tests run locally before review." }] },
    { t: Date.UTC(2026, 10, 17, 8, 40), event: "pack", project: "harbor-web", memories: [
      { id: 88, kind: "fact", scope: "harbor-web", observed_at: "2026-11-09", text: "harbor-web deploys from the release branch only; main is for review builds." },
      { id: 9, gone: true }] },
  ],
  most_used: [
    { id: 42, times: 31, memory: { kind: "preference", scope: "global", text: "Prefers small pull requests, one per piece of work, with tests run locally before merging." } },
    { id: 17, times: 24, memory: { kind: "decision", scope: "atlas-api", text: "The API keeps cursor pagination; offset pagination was dropped after the slow-page incident in October." } },
    { id: 88, times: 19, memory: { kind: "fact", scope: "harbor-web", text: "harbor-web deploys from the release branch only; main is for review builds." } },
  ],
};

const codeMap = {
  root: "D:/work/atlas-api", origin: "compiler", depth: 2,
  blocks: [
    { name: "src", files: 14, symbols: 212 },
    { name: "crates/http", files: 6, symbols: 96 },
    { name: "crates/store", files: 5, symbols: 74 },
    { name: "crates/common", files: 4, symbols: 41 },
    { name: "tests", files: 7, symbols: 58 },
  ],
  links: [
    { from: "src", to: "crates/http", references: 64 },
    { from: "src", to: "crates/store", references: 38 },
    { from: "crates/http", to: "crates/common", references: 22 },
    { from: "crates/store", to: "crates/common", references: 17 },
    { from: "src", to: "crates/common", references: 9 },
  ],
  outside: [
    { package: "serde_json", version: "1.0.140", references: 310, class: null, used_by: ["crates/http", "src"] },
    { package: "sqlx", version: "0.8.6", references: 122, class: "database", used_by: ["crates/store"] },
    { package: "reqwest", version: "0.12.22", references: 48, class: "network", used_by: ["crates/http"] },
    { package: "async-openai", version: "0.29.0", references: 21, class: "model", used_by: ["src"] },
    { package: "tokio", version: "1.47.1", references: 19, class: null, used_by: ["crates/http", "src", "crates/store"] },
  ],
};

const code = {
  root: "D:/work/atlas-api", origin: "compiler", stale: [],
  symbols: [{
    name: "fetch_page", kind: "function", path: "crates/store/src/pages.rs", lines: [41, 88], references: 3,
    signature: "pub async fn fetch_page(db: &Pool, cursor: Option<Cursor>, limit: u32) -> Result<Page>",
    callers: [
      { name: "list_orders", kind: "function", path: "src/routes/orders.rs", lines: [57] },
      { name: "list_invoices", kind: "function", path: "src/routes/invoices.rs", lines: [33] },
      { name: "export_all", kind: "function", path: "src/jobs/export.rs", lines: [120] },
    ],
    callees: [
      { name: "decode_cursor", kind: "function", path: "crates/common/src/cursor.rs", lines: [52] },
      { name: "Page", kind: "type", path: "crates/store/src/pages.rs", lines: [80] },
    ],
  }],
};

const day = (back, requests, output, read, price) => ({
  date: iso(back), requests, output, cache_read: read, input: requests * 40, cache_write_5m: 0,
  cache_write_1h: requests * 900, api_price_usd: price, unpriced_requests: 0,
});
const usageDays = [day(6, 410, 380000, 61e6, 21.4), day(5, 980, 910000, 140e6, 52.9), day(4, 0, 0, 0, 0),
  day(3, 640, 600000, 95e6, 33.1), day(2, 1210, 1150000, 188e6, 71.6), day(1, 870, 790000, 133e6, 49.2),
  day(0, 520, 470000, 79e6, 28.8)];
const sum = (k) => usageDays.reduce((s, d) => s + d[k], 0);
const usage = {
  days: 7, prices: "2026-11-16", note: "",
  totals: Object.fromEntries(["requests", "output", "cache_read", "input", "cache_write_5m", "cache_write_1h", "api_price_usd",
    "unpriced_requests"].map((k) => [k, sum(k)])),
  by_day: usageDays.filter((d) => d.requests > 0),
  by_model: [
    { model: "claude-opus-5-5", requests: 2600, api_price_usd: 171.2 },
    { model: "claude-sonnet-5-5", requests: 1450, api_price_usd: 63.4 },
    { model: "gpt-6.1-terra", requests: 580, api_price_usd: 22.4 },
  ],
  by_agent: [{ agent: "claude-code", requests: 4050 }, { agent: "codex", requests: 580 }],
  unpriced_models: [],
  measured_savings: { source: "rtk gain --daily", rtk: { saved_tokens: 419000, before_tokens: 2998000, after_tokens: 2579000, saved_share: 0.14, commands: 6400, by_day: [] } },
};

export const fixtures = {
  ok: { status, projects, activity, "models list": models, "code-map": codeMap, code, usage, "config set": {},
    "find --": { query: "pagination", products: [
      { product: "memory", state: "ok", results: [
        { id: 17, title: "The API keeps cursor pagination; offset pagination was dropped after the slow-count incident.", detail: "fact · atlas-api · 2026-11-12" },
        { id: 42, title: "Prefers small pull requests, one per piece of work, with tests run locally before review.", detail: "preference · global · 2026-11-10" }] },
      { product: "library", state: "ok", corpora: 1, problems: [], results: [
        { id: 311, title: "Pagination", detail: "docs/api/guide.md:84", snippet: "Pass the cursor from the previous page; pages hold 50 items by default.", source: "D:/docs/atlas.db" }] },
      { product: "code", state: "not_a_symbol", results: [] }] } },
  // A download in progress: the install never answers and `jobs` reports its bytes.
  installing: {
    status, projects, activity, "models list": models,
    jobs: { jobs: [{ kind: "models.install", label: "jina-v5-nano model.gguf", state: "running", unit: "bytes", done: 97_000_000, total: 232_883_776 }], total: 1, hidden: 0 },
  },
  // One product switched off and one left out of the build: the switch's two other states.
  off: {
    status: { ...status, products: { ...status.products, code: { built: false }, library: { built: true, enabled: false } } },
    projects, activity, "config set": {},
    "find --": { query: "zebra", products: [
      { product: "memory", state: "nothing_found", results: [] },
      { product: "library", state: "off", results: [] },
      { product: "code", state: "not_built", results: [] }] },
  },
  empty: {
    status: {
      ...status,
      products: {
        code: { built: true, graphs: 0, rust_indexer: false },
        library: { built: true },
        memory: { built: true, enabled: true, memories: 0, saved_last_7_days: 0, recalls_last_7_days: 0, agents: [] },
      },
      models: { ...models, models: models.models.map((m) => ({ ...m, installed: false, bytes: undefined })) },
    },
    projects: { projects: [] },
    activity: { activity_log: false, days: [], most_used: [] },
    usage: { ...usage, totals: Object.fromEntries(Object.keys(usage.totals).map((k) => [k, 0])), by_day: [], by_model: [], by_agent: [],
      measured_savings: { source: "rtk", available: false, reason: "RTK is not installed (rtk not on PATH)" } },
    "models list": { ...models, models: models.models.map((m) => ({ ...m, installed: false, bytes: undefined })) },
  },
};
