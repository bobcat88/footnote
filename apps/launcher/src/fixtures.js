// Invented sample data, shaped like the engine's real JSON, for browser screenshots only (`?mock=1`). Never shown in the app.
const models = {
  folder: "C:/Users/sample/AppData/Local/footnote/models",
  memory_uses_from_elsewhere: [],
  models: [
    { name: "bekko-a8m", installed: true, bytes: 164462521, license: "MIT", role: "default pair, multilingual half", used_by_memory: true },
    { name: "gte-small", installed: true, bytes: 34726087, license: "MIT", role: "default pair, English half", used_by_memory: true },
    { name: "e5-small", installed: false, license: "MIT", role: "earlier model, tests and comparison", used_by_memory: false },
    { name: "jina-v5-nano", installed: false, license: "CC-BY-NC-4.0 (no commercial use)", role: "French or mixed corpora, opt-in (llama build)", used_by_memory: false },
  ],
};

const status = {
  version: "0.1.0-alpha.1",
  models,
  products: {
    code: { built: true, graphs: 2, rust_indexer: true },
    library: { built: true },
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
  most_used: [
    { id: 42, times: 31, memory: { kind: "preference", scope: "global", text: "Prefers small pull requests, one per piece of work, with tests run locally before merging." } },
    { id: 17, times: 24, memory: { kind: "decision", scope: "atlas-api", text: "The API keeps cursor pagination; offset pagination was dropped after the slow-page incident in October." } },
    { id: 88, times: 19, memory: { kind: "fact", scope: "harbor-web", text: "harbor-web deploys from the release branch only; main is for review builds." } },
  ],
};

export const fixtures = {
  ok: { status, projects, activity, "models list": models },
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
    "models list": { ...models, models: models.models.map((m) => ({ ...m, installed: false, bytes: undefined })) },
  },
};
