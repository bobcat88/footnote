import { useCallback, useEffect, useState } from "react";
import Code from "./Code.jsx";
import { switchTo } from "./Mini.jsx";
import { Activity as ActivityScreen, Models, Projects as ProjectsScreen, Usage } from "./Screens.jsx";
import { AGENTS, footnote, load, mb, num, seen, shortDate } from "./data.js";

const SCREENS = {
  overview: { title: "Overview", note: "this week" },
  code: { title: "Code" },
  projects: { title: "Projects" },
  models: { title: "Models" },
  activity: { title: "Activity", note: "what Memory did" },
  usage: { title: "Usage", note: "tokens your agents used" },
  enterprise: { title: "Footnote Enterprise" },
};

/** `#/code/symbol?mock=1` -> ["code", "symbol"]. */
const route = () => {
  const [name, view] = location.hash.replace(/^#\/?/, "").split("?")[0].split("/");
  return name in SCREENS ? [name, view] : ["overview"];
};

export default function App() {
  const [[screen, view], setScreen] = useState(route);
  const [state, setState] = useState({ loading: true });

  const refresh = useCallback(async () => {
    setState((s) => ({ ...s, loading: true }));
    const r = await load([["status"], ["projects"], ["activity"]]);
    setState({ loading: false, ...r });
  }, []);
  useEffect(() => {
    refresh();
  }, [refresh]);
  useEffect(() => {
    const on = () => setScreen(route());
    addEventListener("hashchange", on);
    return () => removeEventListener("hashchange", on);
  }, []);

  const go = (name) => {
    const query = location.hash.includes("?") ? "?" + location.hash.split("?")[1] : "";
    location.hash = `/${name}${query}`;
  };
  const [status, projects, activity] = state.data ?? [];
  const meta = SCREENS[screen];

  return (
    <div className={screen === "overview" ? "app" : "app no-feed"}>
      <Sidebar status={status} projects={projects} screen={screen} go={go} />
      <main>
        <header className="top">
          <h1>{meta.title}</h1>
          {meta.note && <span>· {meta.note}</span>}
          <div className="end">
            <button className="btn" onClick={() => switchTo("mini")}>Mini mode</button>
            <button className="btn" onClick={refresh} disabled={state.loading}>
              {state.loading ? "Reading…" : "Refresh"}
            </button>
          </div>
        </header>
        {screen === "enterprise" ? (
          <Enterprise />
        ) : state.error ? (
          <Problem error={state.error} retry={refresh} />
        ) : screen === "projects" ? (
          <ProjectsScreen projects={projects} />
        ) : screen === "models" ? (
          <Models />
        ) : screen === "activity" ? (
          <ActivityScreen />
        ) : screen === "usage" ? (
          <Usage />
        ) : screen === "code" ? (
          <Code projects={projects} view={view === "symbol" ? "symbol" : "map"} go={(v) => go(`code/${v}`)} />
        ) : (
          <Overview status={status} projects={projects} refresh={refresh} />
        )}
      </main>
      {screen === "overview" && !state.error && <Activity activity={activity} />}
    </div>
  );
}

function Sidebar({ status, projects, screen, go }) {
  const memory = status?.products?.memory;
  const models = status?.models?.models ?? [];
  const optional = models.filter((m) => !m.installed && m.role?.includes("opt-in"));
  const onDisk = models.reduce((sum, m) => sum + (m.installed ? m.bytes ?? 0 : 0), 0);
  const item = (name, label, extra) => (
    <button className="item" aria-current={screen === name ? "page" : undefined} onClick={() => go(name)}>
      {label}
      {extra}
    </button>
  );
  return (
    <aside className="side">
      <div className="brand">
        <span className="lens" aria-hidden="true">1</span>
        Footnote
        {status && <span className="ver mono">One {status.version.split("-")[0]}</span>}
      </div>
      <h2 className="grp">Products</h2>
      <nav className="nav" aria-label="Products">
        {item("overview", "Overview")}
        <span className="item">
          <span className={live(memory) ? "dot" : "dot off"} />
          Memory<span className="n">{memory ? num(memory.memories) : ""}</span>
        </span>
        <span className="item">
          <span className={live(status?.products?.library) ? "dot" : "dot off"} />
          Library
        </span>
        {item("code", <><span className={live(status?.products?.code) ? "dot" : "dot off"} />Code</>,
          <span className="n">{graphs(status?.products?.code?.graphs)}</span>)}
      </nav>
      <h2 className="grp">Workspace</h2>
      <nav className="nav" aria-label="Workspace">
        {item("projects", "Projects", <span className="n">{projects ? num(projects.projects.length) : ""}</span>)}
        {item("models", "Models",
          <span className="n">{status ? `${models.filter((m) => m.installed).length} / ${models.length}` : ""}</span>)}
        {item("activity", "Activity")}
        {item("usage", "Usage")}
      </nav>
      {optional.length > 0 && (
        <>
          <h2 className="grp">Not installed</h2>
          <nav className="nav" aria-label="Not installed">
            {optional.map((m) => (
              <span key={m.name} className="item ghost" title={`${m.role} · ${m.license}`}>
                <span className="dot off" />
                {m.name}
              </span>
            ))}
          </nav>
        </>
      )}
      <h2 className="grp">Footnote Enterprise</h2>
      <nav className="nav" aria-label="Footnote Enterprise">
        {item("enterprise", "Alias, sign-on…", <span className="n">Learn more</span>)}
      </nav>
      <div className="foot">
        <span>Free and open source</span>
        {onDisk > 0 && <span className="mono">{mb(onDisk)} models</span>}
      </div>
    </aside>
  );
}

/** A product is live when it is in this build and switched on (the dot beside its name). */
const live = (p) => p !== undefined && p.built !== false && p.enabled !== false;

const graphs = (n) => (n === undefined ? "" : n === 1 ? "1 graph" : `${num(n)} graphs`);

function Overview({ status, projects, refresh }) {
  const p = status?.products;
  const memory = p?.memory;
  const code = p?.code;
  const sw = (key, title) => ({ product: key, title, built: p?.[key]?.built, enabled: p?.[key]?.enabled !== false, refresh });
  return (
    <div className="body">
      <div className="row">
        <Card {...sw("memory", "Memory")} big={memory && (memory.built === false ? "–" : num(memory.memories))}
          sub={memory && (memory.built === false ? "" : `${num(memory.saved_last_7_days)} saved · ${num(memory.recalls_last_7_days)} recalls in 7 days`)} />
        <Card {...sw("library", "Library")} big={p && (p.library?.built ? (p.library.enabled === false ? "Off" : "On") : "–")}
          sub={p && (p.library?.built ? "Your folders and docs, searched by any agent" : "")} />
        <Card {...sw("code", "Code")} big={code && (code.built ? num(code.graphs) : "–")}
          sub={code && (!code.built ? "" : code.rust_indexer ? `project ${code.graphs === 1 ? "graph" : "graphs"} · rust-analyzer found`
            : <span className="warn">Install rust-analyzer to build graphs</span>)} />
      </div>
      <h2 className="section">Projects agents worked in</h2>
      <Projects list={projects?.projects} />
      <h2 className="section">Agents</h2>
      <Agents list={memory?.agents} loading={!status} />
    </div>
  );
}

/** A product card. Its switch is the product's on/off setting (`footnote config set <product>.enabled`): off, agents lose its
 * tools at once and Memory's hook goes quiet; nothing is deleted. A company policy can refuse the change; its reason is shown. */
function Card({ product, title, built, enabled, refresh, big, sub }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(null);
  const known = built !== undefined;
  const flip = async () => {
    setBusy(true);
    setError(null);
    try {
      await footnote("config", "set", `${product}.enabled`, String(!enabled));
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className={known && (!built || !enabled) ? "card muted" : "card"}>
      <h2>
        <span className={known && built && enabled ? "dot" : "dot off"} />
        {title}
        {known && (built ? (
          <button type="button" role="switch" aria-checked={enabled} aria-label={`${title} on`} className="switch"
            onClick={flip} disabled={busy}>
            <span aria-hidden="true" />
          </button>
        ) : (
          <span className="state">Not in this build</span>
        ))}
      </h2>
      {big === undefined ? <span className="skel big" /> : <div className="big">{big}</div>}
      {sub === undefined ? <span className="skel" /> : (
        <div className="sub">{known && built && !enabled ? "Off: agents don't get its tools. Nothing is deleted." : sub}</div>
      )}
      {error && <div className="sub warn" role="alert">{error}</div>}
    </section>
  );
}

function Projects({ list }) {
  if (!list) return <span className="skel" />;
  if (list.length === 0)
    return <p className="empty">No project yet. A project appears here when an agent starts a session in it.</p>;
  const warn = (n) => (n > 0 ? "num warn" : "num");
  return (
    <table className="tbl">
      <thead>
        <tr>
          <th scope="col">Project</th>
          <th scope="col">Last seen</th>
          <th scope="col" className="num">Memories</th>
          <th scope="col">Code graph</th>
          <th scope="col" className="num">Uncommitted</th>
          <th scope="col" className="num">Unpushed</th>
        </tr>
      </thead>
      <tbody>
        {list.map((p) => (
          <tr key={p.path}>
            <td title={p.path}>{p.name}</td>
            <td>{p.exists ? seen(p.last_seen) : <span className="warn">folder missing</span>}</td>
            <td className="num">{num(p.memories)}</td>
            <td className={p.code_graph ? "ok" : ""}>{p.code_graph ? "built" : "–"}</td>
            <td className={warn(p.git?.uncommitted)}>{p.git ? num(p.git.uncommitted) : "–"}</td>
            <td className={warn(p.git?.unpushed)}>{p.git ? num(p.git.unpushed) : "–"}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Agents({ list, loading }) {
  if (loading) return <span className="skel" />;
  if (!list?.length)
    return (
      <p className="empty">
        No agent connected. Run <code>footnote install --memory</code> to connect Claude Code, Codex and others.
      </p>
    );
  const onOff = (v) => <td className={v ? "ok" : "warn"}>{v ? "on" : "off"}</td>;
  return (
    <table className="tbl">
      <thead>
        <tr>
          <th scope="col">Agent</th>
          <th scope="col">Memory hooks</th>
          <th scope="col">MCP server</th>
        </tr>
      </thead>
      <tbody>
        {list.map((a) => (
          <tr key={a.agent}>
            <td>{AGENTS[a.agent] ?? a.agent}</td>
            {onOff(a.hooks)}
            {onOff(a.mcp_server)}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Activity({ activity }) {
  const days = (activity?.days ?? []).filter((d) => d.saved || d.prompts_given_memories || d.sessions).reverse();
  return (
    <aside className="feed" aria-label="Activity">
      <h2 className="section">Activity</h2>
      {!activity ? (
        <span className="skel" />
      ) : days.length === 0 ? (
        <p className="dimmed">Nothing yet this week.</p>
      ) : (
        <table className="tbl days">
          <thead>
            <tr>
              <th scope="col">Day</th>
              <th scope="col" className="num">Saved</th>
              <th scope="col" className="num" title="Prompts that were given memories">Recalls</th>
              <th scope="col" className="num">Sessions</th>
            </tr>
          </thead>
          <tbody>
            {days.map((d) => (
              <tr key={d.date}>
                <td className="mono"><time dateTime={d.date}>{shortDate(d.date)}</time></td>
                <td className="num">{num(d.saved)}</td>
                <td className="num">{num(d.prompts_given_memories)}</td>
                <td className="num">{num(d.sessions)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {activity?.most_used?.length > 0 && (
        <>
          <h2 className="section" style={{ marginTop: 8 }}>Most recalled</h2>
          {activity.most_used.slice(0, 6).map((m) => (
            <div className="ev" key={m.id}>
              <span className="mono id">#{m.id}</span>
              <div title={m.memory.text}>
                <span className="clamp">{m.memory.text}</span>
                <span className="dimmed">{num(m.times)} times · {m.memory.scope}</span>
              </div>
            </div>
          ))}
        </>
      )}
    </aside>
  );
}

function Problem({ error, retry }) {
  const missing = error === "ENGINE_MISSING";
  return (
    <div className="body">
      <div className="msg" role="alert">
        <h2>{missing ? "The Footnote engine was not found" : "Footnote could not read its data"}</h2>
        {missing ? (
          <p>
            Install <code>footnote</code>, or set <code>FOOTNOTE_BIN</code> to its path, then refresh.
          </p>
        ) : (
          <p className="mono">{error}</p>
        )}
        <div>
          <button className="btn" onClick={retry}>Refresh</button>
        </div>
      </div>
    </div>
  );
}

function Enterprise() {
  return (
    <div className="body">
      <div className="prose">
        <h2>For teams and companies</h2>
        <p>
          Footnote One stays free and complete for one person or a small team. Footnote Enterprise is a separate build of the
          same engine for companies that need:
        </p>
        <ul>
          <li><b>Alias</b>: names and personal details replaced before text reaches a model.</li>
          <li><b>Company sign-on and rights</b>: who sees which project, from your own directory.</li>
          <li><b>Portfolio</b>: projects across teams in one view, with usage roll-ups.</li>
          <li><b>Curated skills and a compliance kit</b> for regulated work.</li>
        </ul>
        <p>Nothing here is locked in Footnote One, and Footnote One never asks you to upgrade.</p>
      </div>
    </div>
  );
}
