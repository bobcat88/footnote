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
          No code graph yet. In a Rust project, run <code>footnote code-index</code>; its map and symbols then show here.
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
          {[["map", "Architecture"], ["symbol", "Symbol"]].map(([k, label]) => (
            <button key={k} role="tab" aria-selected={view === k} className="tab" onClick={() => go(k)}>
              {label}
            </button>
          ))}
        </div>
      </div>
      {view === "symbol" ? <Symbol root={project.path} key={project.path} /> : <Map root={project.path} key={project.path} />}
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

const CLASS = { database: "database", network: "network", model: "model", "standard library": "std" };

function Map({ root }) {
  const { data, error, loading } = useRead(["code-map", "--root", root]);
  const [picked, setPicked] = useState(null);
  if (error) return <Failed error={error} />;
  if (loading || !data) return <span className="skel" />;
  const blocks = data.blocks.filter((b) => b.symbols > 0);
  const links = data.links.filter((l) => blocks.some((b) => b.name === l.from) && blocks.some((b) => b.name === l.to));
  const { pos, height, width } = place(blocks, links, 860);
  const max = Math.max(1, ...links.map((l) => l.references));
  const sel = picked && blocks.find((b) => b.name === picked);
  const touches = (l) => !sel || l.from === sel.name || l.to === sel.name;
  return (
    <div className="split">
      <figure className="map" aria-label="Architecture map">
        <svg viewBox={`0 0 ${width} ${height}`} width="100%" style={{ maxWidth: width }}>
          <defs>
            <marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="8" markerHeight="8" markerUnits="userSpaceOnUse"
              orient="auto">
              <path d="M0 0 L8 4 L0 8 z" className="arrowhead" />
            </marker>
          </defs>
          {links.map((l) => {
            const a = pos[l.from], b = pos[l.to];
            const [x1, y1, x2, y2] = [a.x + a.w / 2, a.y + a.h, b.x + b.w / 2, b.y];
            const up = y2 <= y1; // a link back up a row (a cycle) bends out to the side
            const d = up
              ? `M${x1} ${a.y + a.h / 2} C${x1 + 140} ${a.y}, ${x2 + 140} ${b.y + b.h}, ${x2 + b.w / 2} ${b.y + b.h / 2}`
              : `M${x1} ${y1} C${x1} ${(y1 + y2) / 2}, ${x2} ${(y1 + y2) / 2}, ${x2} ${y2 - 2}`;
            return (
              <path key={l.from + l.to} d={d} className={touches(l) ? "link on" : "link"} markerEnd="url(#arrow)"
                strokeWidth={1 + 3 * Math.sqrt(l.references / max)}>
                <title>{`${l.from} uses ${l.to}: ${num(l.references)} references`}</title>
              </path>
            );
          })}
          {blocks.map((b) => {
            const p = pos[b.name];
            return (
              <g key={b.name} className={picked === b.name ? "block on" : "block"} tabIndex={0} role="button"
                aria-pressed={picked === b.name}
                onClick={() => setPicked(picked === b.name ? null : b.name)}
                onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), setPicked(picked === b.name ? null : b.name))}>
                <rect x={p.x} y={p.y} width={p.w} height={p.h} rx="8" />
                <text x={p.x + 12} y={p.y + 23} className="name">{b.name}</text>
                <text x={p.x + 12} y={p.y + 42} className="meta">{num(b.symbols)} symbols · {num(b.files)} files</text>
              </g>
            );
          })}
        </svg>
        <figcaption className="dimmed">
          From the compiler&apos;s index. Arrows point from a block to what it uses; thicker means more references.
          {data.depth ? ` Blocks are folders ${data.depth} deep.` : ""}
        </figcaption>
      </figure>
      <aside className="panel">
        {sel ? <BlockDetail block={sel} links={links} outside={data.outside} /> : <Outside list={data.outside} />}
      </aside>
    </div>
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
      <p className="dimmed">{num(block.symbols)} symbols in {num(block.files)} files</p>
      {list(uses, "Uses", (l) => l.to)}
      {list(usedBy, "Used by", (l) => l.from)}
      {list(pkgs, "Outside packages", (o) => o.package)}
    </>
  );
}

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
