import { useEffect, useState } from "react";
import { cancelInstall, footnote, mb, num, seen, shortDate } from "./data.js";

// The workspace screens: Projects, Models, Activity and Usage. Each reads its own engine command; nothing is written.

function useRead(args, round = 0) {
  const [state, setState] = useState({ loading: true });
  const key = args.join(" ");
  useEffect(() => {
    let live = true;
    setState({ loading: true });
    footnote(...args).then(
      (data) => live && setState({ data }),
      (error) => live && setState({ error: String(error) }),
    );
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, round]);
  return state;
}

function Range({ days, setDays, options = [7, 30, 90] }) {
  return (
    <div className="tabs" role="tablist" aria-label="Period">
      {options.map((d) => (
        <button key={d} role="tab" aria-selected={days === d} className="tab" onClick={() => setDays(d)}>
          {d === 1 ? "1 day" : `${d} days`}
        </button>
      ))}
    </div>
  );
}

const Failed = ({ error }) => (
  <p className="empty warn" role="alert">{error === "ENGINE_MISSING" ? "The Footnote engine was not found." : error}</p>
);

export function Projects({ projects }) {
  const list = projects?.projects;
  if (!list) return <div className="body"><span className="skel" /></div>;
  return (
    <div className="body">
      <p className="dimmed">
        Every folder where an agent started a session with Footnote Memory on. Git counts are read when this screen opens.
      </p>
      {list.length === 0 ? (
        <p className="empty">No project yet. A project appears here when an agent starts a session in it.</p>
      ) : (
        <table className="tbl">
          <thead>
            <tr>
              <th scope="col">Project</th>
              <th scope="col">Folder</th>
              <th scope="col">First seen</th>
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
                <td>{p.name}</td>
                <td className="mono" title={p.path}>{p.exists ? p.path : <span className="warn">{p.path} (missing)</span>}</td>
                <td>{shortDate(p.first_seen)}</td>
                <td>{seen(p.last_seen)}</td>
                <td className="num">{num(p.memories)}</td>
                <td className={p.code_graph ? "ok" : ""}>{p.code_graph ? "built" : "–"}</td>
                <td className={p.git?.uncommitted > 0 ? "num warn" : "num"}>{p.git ? num(p.git.uncommitted) : "–"}</td>
                <td className={p.git?.unpushed > 0 ? "num warn" : "num"}>{p.git ? num(p.git.unpushed) : "–"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

export function Models() {
  const [round, setRound] = useState(0);
  const { data, error } = useRead(["models", "list"], round);
  const [busy, setBusy] = useState(null); // { name, progress: {done, total}, error, cancelled }
  // While a download runs, read its progress from the engine's job list (the install itself answers only at the end).
  useEffect(() => {
    if (!busy?.name || busy.error || busy.cancelled) return;
    const timer = setInterval(() => {
      footnote("jobs").then((j) => {
        const job = j.jobs.find((x) => x.kind === "models.install" && x.state === "running" && x.label.startsWith(busy.name + " "));
        if (job) setBusy((b) => b && b.name === busy.name && { ...b, progress: { done: job.done, total: job.total, file: job.label } });
      }, () => {});
    }, 500);
    return () => clearInterval(timer);
  }, [busy?.name, busy?.error, busy?.cancelled]);
  const install = (name) => {
    setBusy({ name });
    footnote("models", "install", name).then(
      () => { setBusy(null); setRound((r) => r + 1); },
      (e) => setBusy((b) => (b?.cancelled ? b : { name, error: String(e) })),
    );
  };
  const cancel = () => {
    setBusy((b) => b && { ...b, cancelled: true });
    cancelInstall();
  };
  if (error) return <div className="body"><Failed error={error} /></div>;
  if (!data) return <div className="body"><span className="skel" /></div>;
  return (
    <div className="body">
      <p className="dimmed">
        Embedding models run on this computer. Installing one is the only step that uses the network: it downloads the pinned
        files from Hugging Face and checks their hashes.
      </p>
      <table className="tbl">
        <thead>
          <tr>
            <th scope="col">Model</th>
            <th scope="col">Role</th>
            <th scope="col">Licence</th>
            <th scope="col">State</th>
            <th scope="col" className="num">Size</th>
          </tr>
        </thead>
        <tbody>
          {data.models.map((m) => (
            <tr key={m.name}>
              <td className="mono">{m.name}</td>
              <td>{m.role}</td>
              <td className={/no commercial/i.test(m.license) ? "warn" : ""}>{m.license}</td>
              <td>
                {m.installed ? (
                  <span className="ok">{m.used_by_memory ? "installed · used by Memory" : "installed"}</span>
                ) : busy?.name === m.name ? (
                  <Download busy={busy} total={m.download_bytes} cancel={cancel} retry={() => install(m.name)} />
                ) : (
                  <button className="btn" type="button" disabled={!!busy && !busy.error && !busy.cancelled}
                    onClick={() => install(m.name)} aria-label={`Install ${m.name}, ${m.download_bytes ? mb(m.download_bytes) : "size unknown"}`}>
                    Install{m.download_bytes ? ` · ${mb(m.download_bytes)}` : ""}
                  </button>
                )}
              </td>
              <td className="num">{m.installed && m.bytes ? mb(m.bytes) : m.download_bytes ? <span className="dimmed">{mb(m.download_bytes)}</span> : "–"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="dimmed mono">{data.folder}</p>
    </div>
  );
}

/** One model's download: progress of the file being fetched, then the outcome. A cancelled or failed install leaves no model
 * behind (the engine renames each file only once it is complete and its hash checked). */
function Download({ busy, total, cancel, retry }) {
  if (busy.cancelled) return <span className="dimmed">Cancelled. Nothing was installed. <button className="btn" type="button" onClick={retry}>Install again</button></span>;
  if (busy.error) return <span className="warn" role="alert">{busy.error} <button className="btn" type="button" onClick={retry}>Try again</button></span>;
  const p = busy.progress;
  const pct = p?.total ? Math.min(100, Math.round((100 * p.done) / p.total)) : null;
  return (
    <span className="dl">
      <progress max={100} value={pct ?? undefined} aria-label={`Downloading ${busy.name}`} />
      <span className="mono dimmed">{p ? `${mb(p.done)}${p.total ? ` of ${mb(p.total)}` : ""}` : total ? `starting · ${mb(total)}` : "starting"}</span>
      <button className="btn" type="button" onClick={cancel}>Cancel</button>
    </span>
  );
}

export function Activity() {
  const [days, setDays] = useState(30);
  const { data, error } = useRead(["activity", "--days", String(days)]);
  const rows = (data?.days ?? []).slice().reverse();
  return (
    <div className="body">
      <div className="bar">
        <Range days={days} setDays={setDays} />
        {data?.note && <span className="dimmed">{data.note}</span>}
      </div>
      {error ? <Failed error={error} /> : !data ? <span className="skel" /> : (
        <div className="split">
          <table className="tbl">
            <thead>
              <tr>
                <th scope="col">Day</th>
                <th scope="col" className="num">Memories saved</th>
                <th scope="col" className="num">Prompts given memories</th>
                <th scope="col" className="num">Sessions</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((d) => (
                <tr key={d.date}>
                  <td className="mono">{shortDate(d.date)}</td>
                  <td className="num">{num(d.saved)}</td>
                  <td className="num">{num(d.prompts_given_memories)}</td>
                  <td className="num">{num(d.sessions)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <aside className="panel">
            <h2 className="title">Most recalled</h2>
            {data.most_used.length === 0 ? <p className="dimmed">No memory recalled yet.</p> : (
              <ul className="plain">
                {data.most_used.map((m) => (
                  <li key={m.id} className="place" title={m.memory.text}>
                    <span><span className="mono id">#{m.id}</span> {m.memory.text}</span>
                    <span className="dimmed">{num(m.times)} times · {m.memory.kind} · {m.memory.scope}</span>
                  </li>
                ))}
              </ul>
            )}
          </aside>
        </div>
      )}
      {data?.recent?.length > 0 && (
        <>
          <h2 className="section">Recent recalls</h2>
          <ul className="hits" aria-label="Recent recalls, newest first">
            {data.recent.map((r) => (
              <li key={`${r.t}-${r.event}`}>
                <span className="dimmed small">
                  {new Date(r.t).toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })}
                  {" · "}{r.event === "pack" ? "session start" : "prompt"}{r.project ? ` · ${r.project}` : ""}
                </span>
                {r.memories.map((m) => (
                  <span key={m.id} className="t">
                    <span className="mono id">#{m.id}</span>{" "}
                    {m.gone ? <span className="dimmed">no longer in memory</span> : <>{m.text} <span className="dimmed small">{m.kind} · {m.scope} · {m.observed_at}</span></>}
                  </span>
                ))}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

const usd = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", maximumFractionDigits: 2 });
const tokens = (n) => (n >= 1e9 ? `${(n / 1e9).toFixed(1)} B` : n >= 1e6 ? `${(n / 1e6).toFixed(1)} M` : num(n));
const AGENT = { "claude-code": "Claude Code", codex: "Codex" };

/** Measured savings only (goal 19): what RTK trimmed from command output before agents read it. Not priced, never added to the
 * totals: a trimmed result would also have been re-read at cache rates later, so any money figure would be an estimate. */
function Trimmed({ s }) {
  const r = s?.rtk;
  return (
    <section className="card">
      <h2>Trimmed before agents read it</h2>
      {r ? (
        <>
          <div className="big">{tokens(r.saved_tokens)}</div>
          <div className="sub">
            {Math.round(100 * r.saved_share)} % of {tokens(r.before_tokens)} command output, measured by RTK. Not priced.
          </div>
        </>
      ) : (
        <div className="sub">{s?.reason ?? "No measured savings for this period."}</div>
      )}
      {s?.footnote?.calls > 0 && (
        <div className="sub">
          Footnote's trimmed search results: {mb(s.footnote.saved_bytes) === "0 MB" ? `${num(Math.round(s.footnote.saved_bytes / 1000))} kB` : mb(s.footnote.saved_bytes)} fewer over {num(s.footnote.calls)} {s.footnote.calls === 1 ? "search" : "searches"}, about {tokens(s.footnote.saved_tokens_estimate)} tokens.
        </div>
      )}
    </section>
  );
}

export function Usage() {
  const [days, setDays] = useState(7);
  const { data, error } = useRead(["usage", "--days", String(days)]);
  const t = data?.totals;
  const daysList = (data?.by_day ?? []).slice().reverse();
  const top = Math.max(1, ...daysList.map((d) => d.api_price_usd));
  return (
    <div className="body">
      <div className="bar">
        <Range days={days} setDays={setDays} options={[1, 7, 30]} />
        <span className="dimmed">From Claude Code and Codex session files on this computer. Counts only, no text kept.</span>
      </div>
      {error ? <Failed error={error} /> : !data ? <span className="skel" /> : (
        <>
          <div className="row">
            <section className="card">
              <h2>Requests</h2>
              <div className="big">{num(t.requests)}</div>
              <div className="sub">{tokens(t.output)} output tokens</div>
            </section>
            <section className="card">
              <h2>Tokens read from cache</h2>
              <div className="big">{tokens(t.cache_read)}</div>
              <div className="sub">{tokens(t.input)} fresh input · {tokens(t.cache_write_5m + t.cache_write_1h)} written</div>
            </section>
            <section className="card">
              <h2>At API list prices</h2>
              <div className="big">{usd.format(t.api_price_usd)}</div>
              <div className="sub">
                What these tokens cost through the API; a subscription pays its flat fee instead.
                {t.unpriced_requests > 0 && (
                  <span className="warn"> {num(t.unpriced_requests)} {t.unpriced_requests === 1 ? "request" : "requests"} unpriced.</span>
                )}
              </div>
            </section>
            <Trimmed s={data.measured_savings} />
          </div>
          <div className="split">
            <table className="tbl">
              <caption className="sr-only">Usage per day</caption>
              <thead>
                <tr>
                  <th scope="col">Day</th>
                  <th scope="col" className="num">Requests</th>
                  <th scope="col" className="num">Output</th>
                  <th scope="col" className="num">Cache read</th>
                  <th scope="col" className="num">At API prices</th>
                  <th scope="col"><span className="sr-only">Share</span></th>
                </tr>
              </thead>
              <tbody>
                {daysList.map((d) => (
                  <tr key={d.date}>
                    <td className="mono">{shortDate(d.date)}</td>
                    <td className="num">{num(d.requests)}</td>
                    <td className="num">{tokens(d.output)}</td>
                    <td className="num">{tokens(d.cache_read)}</td>
                    <td className="num">{usd.format(d.api_price_usd)}</td>
                    <td className="barcell" aria-hidden="true"><span style={{ width: `${(100 * d.api_price_usd) / top}%` }} /></td>
                  </tr>
                ))}
              </tbody>
            </table>
            <aside className="panel">
              <h2 className="title">By model</h2>
              <ul className="plain">
                {data.by_model.slice().sort((a, b) => b.api_price_usd - a.api_price_usd).map((m) => (
                  <li key={m.model}>
                    <span className="mono">{m.model}</span>
                    <span className="mono dimmed">{usd.format(m.api_price_usd)}</span>
                  </li>
                ))}
              </ul>
              <h3 className="section">By agent</h3>
              <ul className="plain">
                {data.by_agent.map((a) => (
                  <li key={a.agent}>
                    <span>{AGENT[a.agent] ?? a.agent}</span>
                    <span className="mono dimmed">{num(a.requests)} requests</span>
                  </li>
                ))}
              </ul>
              <p className="dimmed">Prices from the providers&apos; pages, {data.prices.slice(0, 10)}. Dates in UTC.</p>
            </aside>
          </div>
        </>
      )}
    </div>
  );
}
