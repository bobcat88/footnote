import { useEffect, useRef, useState } from "react";
import { footnote } from "./data.js";

// Search (goal 13): one query over every product, through `footnote find`. Each product answers in its own section, with its
// results or the reason it has none, so an empty Library never reads as "nothing in Memory either".

const NAMES = { memory: "Memory", library: "Library", code: "Code" };
const WHY = {
  off: "Switched off. Turn it on from Overview.",
  not_built: "Not in this build.",
  nothing_found: "Nothing found.",
  not_a_symbol: <>Code looks up one symbol name, such as <code>lookup</code>.</>,
};
const EMPTY = {
  memory: "No memories yet. Agents save them as you work.",
  library: <>No corpus yet. Index a folder with <code>footnote index</code>.</>,
  code: <>No code graph yet. Build one with <code>footnote code-index</code>.</>,
};

export default function Search() {
  const [query, setQuery] = useState("");
  const [asked, setAsked] = useState(null);
  const [state, setState] = useState({});
  const box = useRef(null);
  useEffect(() => box.current?.focus(), []);
  useEffect(() => {
    if (!asked) return;
    let live = true;
    setState({ loading: true });
    footnote("find", "--", asked).then(
      (data) => live && setState({ data }),
      (e) => live && setState({ error: String(e) }),
    );
    return () => { live = false; };
  }, [asked]);

  return (
    <div className="body">
      <form className="ask" role="search" onSubmit={(e) => { e.preventDefault(); const q = query.trim(); if (q) setAsked(q); }}>
        <label htmlFor="q" className="sr-only">Search Memory, Library and Code</label>
        <input id="q" ref={box} className="input" placeholder="Search your memories, documents and code" value={query}
          onChange={(e) => setQuery(e.target.value)} autoComplete="off" spellCheck="false" maxLength={500} />
        <button className="btn" type="submit" disabled={!query.trim() || state.loading}>Search</button>
      </form>
      {state.error ? (
        <p className="empty warn" role="alert">{state.error === "ENGINE_MISSING" ? "The Footnote engine was not found." : state.error}</p>
      ) : state.loading ? (
        <span className="skel" />
      ) : !state.data ? (
        <p className="dimmed">One search over every product that is on. Code needs an exact symbol name.</p>
      ) : (
        <div aria-live="polite">
          {state.data.products.map((p) => <Product key={p.product} p={p} />)}
        </div>
      )}
    </div>
  );
}

function Product({ p }) {
  const n = p.results.length;
  return (
    <section className="found">
      <h2 className="section">
        {NAMES[p.product] ?? p.product}
        {n > 0 && <span className="dimmed"> · {n} {n === 1 ? "result" : "results"}</span>}
      </h2>
      {n > 0 ? (
        <ul className="hits">
          {p.results.map((r, i) => (
            <li key={`${r.source ?? ""}${r.id ?? r.detail}${i}`}>
              <span className="t">{r.title}</span>
              <span className="mono dimmed d">{r.detail}</span>
              {r.snippet && r.snippet !== r.title && <span className="s">{r.snippet}</span>}
            </li>
          ))}
        </ul>
      ) : p.state === "error" ? (
        <p className="empty warn" role="alert">{p.error}</p>
      ) : (
        <p className="empty">{p.state === "empty" ? EMPTY[p.product] : WHY[p.state] ?? p.state}</p>
      )}
      {p.problems?.length > 0 && (
        <p className="warn small">{p.problems.length} {p.problems.length === 1 ? "corpus" : "corpora"} could not be read: {p.problems.map((x) => x.source).join(", ")}</p>
      )}
    </section>
  );
}
