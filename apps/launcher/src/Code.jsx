import { useEffect, useState } from "react";
import { footnote, num } from "./data.js";
import { place } from "./layout.js";

// Code, by zoom level (DESIGN.md section 11): the architecture map of a project, then one symbol's callers and callees.
// Never the whole graph at once.
export default function Code({ projects, view, go }) {
  const graphs = (projects?.projects ?? []).filter((p) => p.code_graph && p.exists);
  const [root, setRoot] = useState(null);
  const project = graphs.find((p) => p.path === root) ?? graphs[0];

  if (!projects) return <div className="body"><span className="skel" /></div>;
  if (!project)
    return (
      <div className="body">
        <p className="empty">
          No code graph yet. In a project folder, run <code>footnote code-index</code>; its map and symbols then show here.
        </p>
      </div>
    );
  return (
    <div className="body">
      <div className="bar">
        {graphs.length > 1 ? (
          <label>
            <span className="sr-only">Project</span>
            <select className="btn" value={project.path} onChange={(e) => setRoot(e.target.value)}>
              {graphs.map((p) => <option key={p.path} value={p.path}>{p.name}</option>)}
            </select>
          </label>
        ) : (
          <b>{project.name}</b>
        )}
        <div className="tabs" role="tablist" aria-label="Code view">
          {[["map", "Architecture"], ["layers", "Layers"], ["symbol", "Symbol"]].map(([k, label]) => (
            <button key={k} role="tab" aria-selected={view === k} className="tab" onClick={() => go(k)}>
              {label}
            </button>
          ))}
        </div>
      </div>
      {view === "symbol" ? <Symbol root={project.path} key={project.path} /> : view === "layers" ? <Layers root={project.path} key={project.path} /> : <Map root={project.path} key={project.path} />}
    </div>
  );
}

function useRead(args) {
  const [state, setState] = useState({});
  const key = args?.join(" ");
  useEffect(() => {
    if (!args) return;
    let live = true;
    setState({ loading: true });
    footnote(...args).then(
      (data) => live && setState({ data }),
      (error) => live && setState({ error: String(error) }),
    );
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return state;
}

// What the project calls out to, grouped by kind (DESIGN.md section 11, blueprint map of 17 Nov 2026). `server` and `mcp`
// packages mean the project is called in, so they sit on the "calls in" side.
const KINDS = [
  ["database", "Databases"],
  ["model", "Model runtimes"],
  ["network", "Network and APIs"],
  ["system", "Files and system"],
];
const INBOUND = { server: "Serves HTTP", mcp: "Serves MCP" };

/** Flows: from each entry block, follow the heaviest link down until a block uses nothing more, then its top outside
 *  package. A guess at how a request travels, labelled as such; at most four. */
function flows(blocks, links, outside) {
  const names = new Set(blocks.map((b) => b.name));
  return blocks
    .filter((b) => b.entry && links.some((l) => l.from === b.name))
    .map((b) => {
      const path = [b.name];
      for (;;) {
        const next = links
          .filter((l) => l.from === path[path.length - 1] && names.has(l.to) && !path.includes(l.to))
          .sort((x, y) => y.references - x.references)[0];
        if (!next) break;
        path.push(next.to);
      }
      const last = path[path.length - 1];
      const pkg = outside.filter((o) => o.class && !INBOUND[o.class] && o.used_by.includes(last))[0];
      return { name: `From ${b.name}`, path, pkg: pkg?.package };
    })
    .slice(0, 4);
}

function Map({ root }) {
  const { data, error, loading } = useRead(["code-map", "--root", root]);
  const [picked, setPicked] = useState(null);
  const [flow, setFlow] = useState(0);
  // Impact radius of one symbol (`code-impact`): the blocks its change reaches light up, the panel lists who and what.
  const [ask, setAsk] = useState("");
  const [impact, setImpact] = useState(null);
  const runImpact = (name) => {
    setPicked(null);
    setImpact({ name, loading: true });
    footnote("code-impact", name, "--root", root).then(
      (data) => setImpact({ name, data }),
      (err) => setImpact({ name, error: String(err) }),
    );
  };
  if (error) return <Failed error={error} />;
  if (loading || !data) return <span className="skel" />;
  const blocks = data.blocks.filter((b) => b.symbols > 0);
  const links = data.links.filter((l) => blocks.some((b) => b.name === l.from) && blocks.some((b) => b.name === l.to));
  const { pos, height, width } = place(blocks, links, 760);
  const max = Math.max(1, ...links.map((l) => l.references));
  const sel = picked && blocks.find((b) => b.name === picked);
  const all = flows(blocks, links, data.outside);
  const lit = picked || impact ? null : all[flow];
  // Blocks the change reaches, and the block that defines the symbol itself.
  const hit = new Set([
    ...(impact?.data?.blocks ?? []),
    ...(impact?.data?.found ?? []).map((f) => f.path.split("/").slice(0, -1).slice(0, data.depth || 2).join("/") || "."),
  ]);
  const onPath = (l) => lit && lit.path.some((n, i) => n === l.from && lit.path[i + 1] === l.to);
  const touches = (l) => (sel ? l.from === sel.name || l.to === sel.name : onPath(l));
  const inbound = data.outside.filter((o) => INBOUND[o.class]);
  const groups = KINDS.map(([k, label]) => [label, data.outside.filter((o) => o.class === k).slice(0, 4)]).filter(([, l]) => l.length);
  const rest = data.outside.filter((o) => !o.class || o.class === "standard library");

  // The outside column, beside the code zone: one row per package under its kind's heading.
  const ox = width + 64, rows = [];
  let oy = 28;
  for (const [label, list] of groups) {
    rows.push({ head: label, y: oy });
    oy += 22;
    for (const o of list) {
      rows.push({ o, y: oy });
      oy += 44;
    }
    oy += 10;
  }
  const W = ox + 250, H = Math.max(height + 40, oy + 10);
  const pkgY = Object.fromEntries(rows.filter((r) => r.o).map((r) => [r.o.package, r.y]));
  const litPkg = lit?.pkg;

  return (
    <div className="split blueprint-split">
      <nav className="panel flows" aria-label="Flows">
        <h3 className="section">Before you change…</h3>
        <form className="impact-ask" onSubmit={(e) => { e.preventDefault(); const n = ask.trim(); if (n) runImpact(n); }}>
          <label className="sr-only" htmlFor="impact-name">Symbol to change</label>
          <input id="impact-name" className="btn" placeholder="a function or type" value={ask} onChange={(e) => setAsk(e.target.value)} />
          <button type="submit" className="btn">Impact</button>
        </form>
        {impact && (
          <button type="button" className="btn ghost" onClick={() => { setImpact(null); setAsk(""); }}>
            Clear the impact of {impact.name}
          </button>
        )}
        <h3 className="section">Flows</h3>
        {all.length === 0 && <p className="dimmed">No entry block calls into the others yet.</p>}
        {all.map((f, i) => (
          <button key={f.name} type="button" className={!picked && !impact && i === flow ? "flow on" : "flow"}
            aria-pressed={!picked && !impact && i === flow} onClick={() => { setPicked(null); setImpact(null); setFlow(i); }}>
            <b>{f.name}</b>
            <span className="mono">{[...f.path.map(short), f.pkg].filter(Boolean).join(" → ")}</span>
          </button>
        ))}
        <p className="dimmed note">Each flow follows the heaviest references from a block nothing else calls.</p>
        {inbound.length > 0 && (
          <>
            <h3 className="section">Calls in</h3>
            <ul className="plain">{inbound.map((o) => <li key={o.package}><span>{o.package}</span><span className="dimmed">{INBOUND[o.class]}</span></li>)}</ul>
          </>
        )}
      </nav>
      <figure className="map blueprint" aria-label="Architecture map">
        <svg viewBox={`-24 -40 ${W + 24} ${H + 40}`} width="100%" style={{ maxWidth: W + 24 }}>
          <defs>
            <marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="8" markerHeight="8" markerUnits="userSpaceOnUse" orient="auto">
              <path d="M0 0 L8 4 L0 8 z" className="arrowhead" />
            </marker>
          </defs>
          <rect x={-12} y={-12} width={width + 36} height={height + 24} rx="16" className="zone code" />
          <text x={4} y={-18} className="zone-t">The code</text>
          <rect x={ox - 16} y={-12} width={262} height={H} rx="16" className="zone out" />
          <text x={ox} y={-18} className="zone-t">Calls out</text>
          {links.map((l) => {
            const a = pos[l.from], b = pos[l.to];
            const [x1, y1, x2, y2] = [a.x + a.w / 2, a.y + a.h, b.x + b.w / 2, b.y];
            const up = y2 <= y1;
            const d = up
              ? `M${x1} ${a.y + a.h / 2} C${x1 + 140} ${a.y}, ${x2 + 140} ${b.y + b.h}, ${x2 + b.w / 2} ${b.y + b.h / 2}`
              : `M${x1} ${y1} C${x1} ${(y1 + y2) / 2}, ${x2} ${(y1 + y2) / 2}, ${x2} ${y2 - 2}`;
            const id = `l-${l.from}-${l.to}`.replace(/[^\w-]/g, "_");
            return (
              <g key={id}>
                <path id={id} d={d} className={touches(l) ? "link on" : "link"} markerEnd="url(#arrow)"
                  strokeWidth={1 + 3 * Math.sqrt(l.references / max)}>
                  <title>{`${l.from} uses ${l.to}: ${num(l.references)} references`}</title>
                </path>
                {onPath(l) && !sel && (
                  <circle r="3.2" className="pulse">
                    <animateMotion dur={`${Math.max(0.9, 2.4 - l.references / max)}s`} repeatCount="indefinite"><mpath href={`#${id}`} /></animateMotion>
                  </circle>
                )}
                {touches(l) && (
                  <text x={(x1 + x2) / 2 + 6} y={up ? Math.min(a.y, b.y) - 6 : (y1 + y2) / 2} className="link-t">{num(l.references)}</text>
                )}
              </g>
            );
          })}
          {/* Dashed lines from a block to the outside packages it uses: only for the selected block or flow, else it is a hairball. */}
          {Object.entries(pkgY).map(([pkg, y]) => {
            const o = data.outside.find((x) => x.package === pkg);
            const from = sel ? (o.used_by.includes(sel.name) ? [sel.name] : []) : lit ? (pkg === litPkg ? [lit.path[lit.path.length - 1]] : []) : [];
            return from.filter((n) => pos[n]).map((n) => {
              const p = pos[n];
              return <path key={pkg + n} d={`M${p.x + p.w} ${p.y + p.h / 2} C${ox - 40} ${p.y + p.h / 2}, ${ox - 60} ${y + 14}, ${ox - 4} ${y + 14}`} className="link out on" />;
            });
          })}
          {blocks.map((b) => {
            const p = pos[b.name];
            const n = b.memories?.count ?? 0;
            const reached = !picked && hit.has(b.name);
            const on = picked === b.name || (!picked && lit?.path.includes(b.name)) || reached;
            return (
              <g key={b.name} className={`block${on ? " on" : ""}${reached ? " hit" : ""}`} tabIndex={0} role="button" aria-pressed={picked === b.name}
                aria-label={`${b.name}, ${num(b.symbols)} symbols${n ? `, ${n} memories` : ""}${reached ? ", reached by the change" : ""}`}
                onClick={() => setPicked(picked === b.name ? null : b.name)}
                onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), setPicked(picked === b.name ? null : b.name))}>
                <rect x={p.x} y={p.y} width={p.w} height={p.h} rx="12" />
                <text x={p.x + 12} y={p.y + 23} className="name">{b.name}</text>
                <text x={p.x + 12} y={p.y + 42} className="meta">{num(b.symbols)} symbols · {num(b.files)} files</text>
                {n > 0 && (
                  <g className="pill">
                    <rect x={p.x + p.w - 86} y={p.y - 10} width={78} height={20} rx="10" />
                    <text x={p.x + p.w - 47} y={p.y + 4} textAnchor="middle">{n === 1 ? "1 memory" : `${n} memories`}</text>
                  </g>
                )}
              </g>
            );
          })}
          {rows.map((r) =>
            r.head ? (
              <text key={r.head} x={ox} y={r.y + 4} className="kind">{r.head}</text>
            ) : (
              <g key={r.o.package} className={r.o.package === litPkg || (sel && r.o.used_by.includes(sel.name)) ? "pkg on" : "pkg"}>
                <rect x={ox} y={r.y} width={230} height={36} rx="8" />
                <text x={ox + 12} y={r.y + 22} className="name">{r.o.package}</text>
                <text x={ox + 218} y={r.y + 22} textAnchor="end" className="meta">{num(r.o.references)}</text>
                <title>{`${r.o.package} ${r.o.version} · used by ${r.o.used_by.join(", ")}`}</title>
              </g>
            ),
          )}
          {rest.length > 0 && (
            <text x={ox} y={oy + 8} className="rest">+ {rest.length} libraries with no outside service</text>
          )}
        </svg>
        <figcaption className="dimmed">
          From the compiler&apos;s index. Arrows point from a block to what it uses; thicker means more references.
          {data.depth ? ` Blocks are folders ${data.depth} deep.` : ""}
        </figcaption>
      </figure>
      <aside className="panel">
        {sel ? <BlockDetail block={sel} links={links} outside={data.outside} /> : impact ? <ImpactDetail impact={impact} /> : lit ? <FlowDetail flow={lit} links={links} blocks={blocks} /> : <Outside list={data.outside} />}
      </aside>
    </div>
  );
}

const short = (n) => n.split("/").pop();

/** Layers (the Strata direction, 17 Nov 2026): the same map read top to bottom. Entry blocks (nothing calls them), base blocks
 *  (they call no other block), features between, then the outside packages by kind. Under each layer, its links down. */
function Layers({ root }) {
  const { data, error, loading } = useRead(["code-map", "--root", root]);
  if (error) return <Failed error={error} />;
  if (loading || !data) return <span className="skel" />;
  const blocks = data.blocks.filter((b) => b.symbols > 0);
  const links = data.links.filter((l) => blocks.some((b) => b.name === l.from) && blocks.some((b) => b.name === l.to));
  const calls = (b) => links.some((l) => l.from === b.name);
  const entry = blocks.filter((b) => b.entry);
  const base = blocks.filter((b) => !b.entry && !calls(b));
  const features = blocks.filter((b) => !entry.includes(b) && !base.includes(b));
  const max = Math.max(1, ...links.map((l) => l.references));
  const strata = [
    ["Entry", "Where a run starts: nothing else in the project calls these.", entry],
    ["Features", "What the project does.", features],
    ["Base", "Shared by everything: calls no other block.", base],
  ].filter(([, , list]) => list.length);
  const down = (list) => links.filter((l) => list.some((b) => b.name === l.from)).sort((a, b) => b.references - a.references);
  const kinds = KINDS.map(([k, label]) => [label, data.outside.filter((o) => o.class === k)]).filter(([, l]) => l.length);
  return (
    <div className="layers">
      {strata.map(([title, note, list]) => (
        <section key={title} className="stratum" aria-label={title}>
          <div><h2 className="title">{title}</h2><p className="dimmed">{note}</p></div>
          <div className="cells">
            {list.map((b) => (
              <div key={b.name} className="cell">
                <b className="mono">{b.name}</b>
                <span className="dimmed">{num(b.symbols)} symbols · {num(b.files)} files</span>
                {b.memories?.count > 0 && <span className="mems">{b.memories.count === 1 ? "1 memory" : `${b.memories.count} memories`}</span>}
              </div>
            ))}
          </div>
          {down(list).length > 0 && (
            <ul className="down">
              {down(list).map((l) => (
                <li key={l.from + l.to} className={l.references / max > 0.5 ? "heavy" : ""}>
                  {short(l.from)} → <b>{short(l.to)}</b> {num(l.references)}
                  <i style={{ width: Math.max(4, 90 * (l.references / max)) }} />
                </li>
              ))}
            </ul>
          )}
        </section>
      ))}
      {kinds.length > 0 && (
        <section className="stratum" aria-label="Outside">
          <div><h2 className="title">Outside</h2><p className="dimmed">Services and runtimes the project calls out to.</p></div>
          <div className="cells">
            {kinds.flatMap(([label, list]) => list.map((o) => (
              <div key={o.package} className="cell">
                <b className="mono">{o.package}</b>
                <span className="dimmed">{label.toLowerCase()} · {num(o.references)} refs · {o.used_by.map(short).join(", ")}</span>
              </div>
            )))}
          </div>
        </section>
      )}
      <p className="dimmed">Read top to bottom: each layer calls the ones below it. The longest bars are the heaviest links.</p>
    </div>
  );
}

function Memories({ list }) {
  if (!list?.length) return null;
  return (
    <>
      <h3 className="section">Memories about it</h3>
      {list.map((m) => (
        <div key={m.id} className="mem">
          {m.kind && <span className="mono">{m.kind} · {m.observed_at}</span>}
          <p>{m.text}</p>
        </div>
      ))}
    </>
  );
}

function ImpactDetail({ impact }) {
  if (impact.loading) return <span className="skel" />;
  if (impact.error) return <p className="dimmed">{impact.error}</p>;
  const v = impact.data;
  if (!v.found?.length) return <p className="dimmed">No symbol named <code>{impact.name}</code> in this graph.</p>;
  const row = (c) => (
    <li key={`${c.path}:${c.line}`}>
      <span>
        <span className="lvl">{c.level}</span> {c.name} <span className="dimmed mono">{c.path}:{c.call_line}</span>
      </span>
      <span className={c.origin === "compiler" ? "grade" : "grade guess"}>{c.origin === "compiler" ? "compiler" : "parsed"}</span>
    </li>
  );
  return (
    <>
      <h2 className="title mono">{v.symbol}</h2>
      <p className="dimmed">
        A change reaches {num(v.counts.callers)} callers and {num(v.counts.tests)} tests in {num(v.counts.files)} files, up to {v.depth} levels.
        Lit on the map.
      </p>
      <Memories list={v.memories} />
      {v.tests.length > 0 && (
        <>
          <h3 className="section">Tests to run</h3>
          <ul className="plain impact">{v.tests.map(row)}</ul>
        </>
      )}
      <h3 className="section">Callers, by level</h3>
      {v.callers.length ? <ul className="plain impact">{v.callers.map(row)}</ul> : <p className="dimmed">Nothing in the project calls it.</p>}
    </>
  );
}

function FlowDetail({ flow, links, blocks }) {
  const steps = flow.path.slice(1).map((to, i) => links.find((l) => l.from === flow.path[i] && l.to === to));
  const mems = blocks.filter((b) => flow.path.includes(b.name)).flatMap((b) => b.memories?.recent ?? []).slice(0, 4);
  return (
    <>
      <h2 className="title">{flow.name}</h2>
      <p className="dimmed">Lit on the map: {[...flow.path, flow.pkg].filter(Boolean).join(" → ")}.</p>
      <Memories list={mems} />
      <h3 className="section">Steps</h3>
      <ul className="plain">
        {steps.map((l) => <li key={l.from + l.to}><span>{short(l.from)} → {short(l.to)}</span><span className="mono dimmed">{num(l.references)}</span></li>)}
      </ul>
    </>
  );
}

function BlockDetail({ block, links, outside }) {
  const uses = links.filter((l) => l.from === block.name);
  const usedBy = links.filter((l) => l.to === block.name);
  const pkgs = outside.filter((o) => o.used_by.includes(block.name));
  const list = (items, label, pick) =>
    items.length > 0 && (
      <>
        <h3 className="section">{label}</h3>
        <ul className="plain">
          {items.map((l) => (
            <li key={pick(l)}><span>{pick(l)}</span><span className="mono dimmed">{num(l.references)}</span></li>
          ))}
        </ul>
      </>
    );
  return (
    <>
      <h2 className="title mono">{block.name}</h2>
      <p className="dimmed">{num(block.symbols)} symbols in {num(block.files)} files{block.entry ? "; nothing else in the project calls it" : ""}</p>
      <Memories list={block.memories?.recent} />
      {list(uses, "Uses", (l) => l.to)}
      {list(usedBy, "Used by", (l) => l.from)}
      {list(pkgs, "Outside packages", (o) => o.package)}
    </>
  );
}

const CLASS = { database: "database", network: "network", model: "model", system: "system", server: "serves HTTP", mcp: "serves MCP", "standard library": "std" };

function Outside({ list }) {
  return (
    <>
      <h2 className="title">Outside packages</h2>
      <p className="dimmed">What the project calls that it did not write. Select a block to see its own.</p>
      <table className="tbl">
        <thead>
          <tr><th scope="col">Package</th><th scope="col">Kind</th><th scope="col" className="num">Refs</th></tr>
        </thead>
        <tbody>
          {list.slice(0, 14).map((o) => (
            <tr key={o.package + o.version}>
              <td title={`${o.package} ${o.version} · used by ${o.used_by.join(", ")}`}>{o.package}</td>
              <td>{o.class ? <span className="chip">{CLASS[o.class]}</span> : ""}</td>
              <td className="num">{num(o.references)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}

function Symbol({ root }) {
  const [name, setName] = useState("");
  const [asked, setAsked] = useState(null);
  const { data, error, loading } = useRead(asked && ["code", asked, "--root", root]);
  const [i, setI] = useState(0);
  const found = data?.symbols ?? [];
  const s = found[Math.min(i, found.length - 1)];
  return (
    <div className="split">
      <div className="map">
        <form className="ask" onSubmit={(e) => { e.preventDefault(); const n = name.trim(); if (n) { setI(0); setAsked(n); } }}>
          <label htmlFor="sym" className="sr-only">Symbol name</label>
          <input id="sym" className="input mono" placeholder="A function or type, e.g. lookup" value={name}
            onChange={(e) => setName(e.target.value)} autoComplete="off" spellCheck="false" />
          <button className="btn" type="submit" disabled={!name.trim()}>Find</button>
        </form>
        {error ? <Failed error={error} /> : loading ? <span className="skel" /> : !asked ? (
          <p className="dimmed">Type an exact name. Footnote shows where it is defined, who calls it and what it calls.</p>
        ) : !s ? (
          <p className="empty">Nothing named <code>{asked}</code> in this project&apos;s graph.</p>
        ) : (
          <Ego s={s} />
        )}
      </div>
      <aside className="panel">
        {s && (
          <>
            {found.length > 1 && (
              <label className="dimmed">
                {found.length} definitions:{" "}
                <select className="btn" value={i} onChange={(e) => setI(Number(e.target.value))}>
                  {found.map((f, k) => <option key={k} value={k}>{f.path}:{f.lines[0]}</option>)}
                </select>
              </label>
            )}
            <h2 className="title mono">{s.name}</h2>
            <p className="mono sig">{s.signature || s.kind}</p>
            <p className="mono dimmed">{s.path}:{s.lines[0]}–{s.lines[1]}</p>
            {data.stale?.length > 0 && (
              <p className="warn">Changed since indexing: {data.stale.join(", ")}. Run <code>footnote code-index</code>.</p>
            )}
            <Places label={`Called by · ${s.callers.length}`} items={s.callers} />
            <Places label={`Calls · ${s.callees.length} in project`} items={s.callees} />
          </>
        )}
      </aside>
    </div>
  );
}

function Places({ label, items }) {
  return (
    <>
      <h3 className="section">{label}</h3>
      {items.length === 0 ? <p className="dimmed">None.</p> : (
        <ul className="plain">
          {items.map((c) => (
            <li key={c.name + c.path} className="place">
              <span>{c.name}</span>
              <span className="mono dimmed">{c.path}:{c.lines[0]}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

/** Callers on the left, the symbol in the middle, callees on the right. */
function Ego({ s }) {
  const W = 760, rowH = 52, show = 8;
  const left = s.callers.slice(0, show), right = s.callees.slice(0, show);
  const H = Math.max(3, left.length, right.length) * rowH + 32;
  const ys = (n) => Array.from({ length: n }, (_, k) => H / 2 + (k - (n - 1) / 2) * rowH);
  const box = (x, y, label, cls, key) => (
    <g key={key} className={cls}>
      <rect x={x} y={y - 18} width="170" height="36" rx="8" />
      <text x={x + 12} y={y + 5}>{label.length > 20 ? label.slice(0, 19) + "…" : label}<title>{label}</title></text>
    </g>
  );
  return (
    <figure className="ego">
      <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ maxWidth: W }} role="img"
        aria-label={`${s.name}: ${s.callers.length} callers, ${s.callees.length} callees`}>
        {ys(left.length).map((y, k) => <path key={"l" + k} className="link on" d={`M190 ${y} C260 ${y}, 240 ${H / 2}, 295 ${H / 2}`} />)}
        {ys(right.length).map((y, k) => <path key={"r" + k} className="link on" d={`M465 ${H / 2} C520 ${H / 2}, 500 ${y}, 570 ${y}`} />)}
        {left.map((c, k) => box(20, ys(left.length)[k], c.name, "block", "lb" + k))}
        {box(295, H / 2, s.name, "block on", "me")}
        {right.map((c, k) => box(570, ys(right.length)[k], c.name, "block", "rb" + k))}
      </svg>
      {(s.callers.length > show || s.callees.length > show) && (
        <figcaption className="dimmed">Showing the first {show} on each side; the full lists are on the right.</figcaption>
      )}
    </figure>
  );
}

function Failed({ error }) {
  return <p className="empty warn" role="alert">{error === "ENGINE_MISSING" ? "The Footnote engine was not found." : error}</p>;
}
