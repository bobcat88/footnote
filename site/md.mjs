// Small HTML-to-Markdown converter for our page bodies (site/src/pages/*.html).
// Keeps the Markdown twins and llms-full.txt generated from the exact same HTML source.

const VOID = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"]);
const SKIP = new Set(["svg", "canvas", "button", "script", "style"]);

function decode(s) {
  return s
    .replace(/&nbsp;/g, " ")
    .replace(/&middot;/g, "·")
    .replace(/&mdash;/g, "—")
    .replace(/&ndash;/g, "–")
    .replace(/&apos;|&#39;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&")
    .replace(/&#(\d+);/g, (_, n) => String.fromCharCode(Number(n)))
    .replace(/&#x([0-9a-f]+);/gi, (_, n) => String.fromCharCode(parseInt(n, 16)));
}

function parseAttrs(raw) {
  const attrs = {};
  for (const m of raw.matchAll(/([a-zA-Z0-9:-]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>/]+)))?/g)) {
    attrs[m[1].toLowerCase()] = m[2] !== undefined ? decode(m[2]) : m[3] !== undefined ? decode(m[3]) : m[4] !== undefined ? decode(m[4]) : "";
  }
  return attrs;
}

function parse(html) {
  const clean = html.replace(/<!--[\s\S]*?-->/g, "");
  const root = { tag: "root", attrs: {}, children: [] };
  const stack = [root];
  const re = /<\/?([a-zA-Z][a-zA-Z0-9-]*)([^>]*?)(\/?)>/g;
  let last = 0, m;
  while ((m = re.exec(clean))) {
    if (m.index > last) stack.at(-1).children.push({ text: decode(clean.slice(last, m.index)) });
    last = re.lastIndex;
    const isClose = m[0][1] === "/";
    const tag = m[1].toLowerCase();
    if (isClose) {
      for (let i = stack.length - 1; i > 0; i--) {
        if (stack[i].tag === tag) { stack.length = i; break; }
      }
    } else {
      const node = { tag, attrs: parseAttrs(m[2]), children: [] };
      stack.at(-1).children.push(node);
      if (!m[3] && !VOID.has(tag)) stack.push(node);
    }
  }
  if (last < clean.length) stack.at(-1).children.push({ text: decode(clean.slice(last)) });
  return root;
}

const hasClass = (node, cls) => (node.attrs?.class || "").split(/\s+/).includes(cls);
const isHidden = node => node.attrs?.["aria-hidden"] === "true" || hasClass(node, "lensnote");
const normSpace = s => s.replace(/\s+/g, " ");

function inline(node, mapHref, inHeading = false) {
  if (node.text !== undefined) return normSpace(node.text);
  if (SKIP.has(node.tag) || isHidden(node)) return "";
  if (node.tag === "br") return "\n";
  if (node.tag === "code") {
    const t = node.children.map(c => c.text ?? inline(c, mapHref, inHeading)).join("").trim();
    return "`" + t.replace(/`/g, "\\`") + "`";
  }
  const joiner = hasClass(node, "blocks") ? " " : "";
  const parts = [];
  for (const c of node.children) {
    let chunk = inline(c, mapHref, inHeading);
    if (!chunk) continue;
    if (c.tag === "span" && hasClass(c, "d") && parts.length && !/\s$/.test(parts.at(-1))) {
      chunk = " (" + chunk.trim() + ")";
    }
    if (parts.length) {
      const prev = parts.at(-1);
      if (joiner && !/\s$/.test(prev) && !/^\s/.test(chunk)) parts.push(joiner);
      else if (/\*\*$/.test(prev) && /^[A-Za-z0-9]/.test(chunk)) parts.push(" ");
    }
    parts.push(chunk);
  }
  const body = parts.join("");
  if (node.tag === "a" && node.attrs.href) {
    const label = body.trim();
    return label ? `[${label}](${mapHref(node.attrs.href)})` : "";
  }
  if (node.tag === "b" || node.tag === "strong") {
    const t = body.trim();
    return t ? `**${t}**` : "";
  }
  if (node.tag === "em") {
    const t = body.trim();
    if (!t) return "";
    return inHeading ? t : `*${t}*`;
  }
  return body;
}

function renderTable(node, mapHref) {
  const rows = [];
  const walk = n => {
    if (!n.tag) return;
    if (n.tag === "tr") {
      const cells = n.children.filter(c => c.tag === "th" || c.tag === "td").map(c => inline(c, mapHref).trim().replace(/\|/g, "\\|"));
      if (cells.length) rows.push(cells);
      return;
    }
    for (const c of n.children) walk(c);
  };
  walk(node);
  if (!rows.length) return "";
  const cols = rows[0].length;
  const header = "| " + rows[0].join(" | ") + " |";
  const sep = "|" + Array(cols).fill("---").join("|") + "|";
  const body = rows.slice(1).map(r => "| " + r.join(" | ") + " |");
  return [header, sep, ...body].join("\n");
}

function blocks(node, mapHref, out) {
  if (node.text !== undefined) return;
  if (SKIP.has(node.tag) || isHidden(node)) return;

  if (node.attrs?.role === "img" && node.attrs?.["aria-label"]) {
    out.push(node.attrs["aria-label"].trim());
    return;
  }

  if (hasClass(node, "cmd")) {
    const code = node.children.find(c => c.tag === "code");
    if (code) {
      const cmd = code.children.map(c => c.text ?? "").join("").trim();
      out.push("```sh\n" + cmd + "\n```");
    }
    return;
  }

  if (hasClass(node, "mini")) {
    const items = node.children.filter(c => c.tag === "div").map(d => {
      const b = d.children.find(c => c.tag === "b");
      const s = d.children.find(c => c.tag === "span");
      const bt = b ? inline(b, mapHref).trim() : "";
      const st = s ? inline(s, mapHref).trim() : "";
      return `- **${bt}** — ${st}`;
    });
    if (items.length) out.push(items.join("\n"));
    return;
  }

  if (hasClass(node, "big") && node.tag === "div") {
    const t = inline(node, mapHref).trim();
    if (t) out.push(`**${t}**`);
    return;
  }

  if (node.tag === "h1" || node.tag === "h2" || node.tag === "h3") {
    const lvl = "#".repeat(Number(node.tag[1]));
    const t = inline(node, mapHref, true).trim();
    if (t) out.push(`${lvl} ${t}`);
    return;
  }

  if (node.tag === "p") {
    const t = inline(node, mapHref).trim();
    if (t) out.push(t);
    return;
  }

  if (node.tag === "ul") {
    const items = node.children.filter(c => c.tag === "li").map(li => "- " + inline(li, mapHref).trim()).filter(x => x !== "- ");
    if (items.length) out.push(items.join("\n"));
    return;
  }

  if (node.tag === "table") {
    const tbl = renderTable(node, mapHref);
    if (tbl) out.push(tbl);
    return;
  }

  if (hasClass(node, "cta")) {
    const links = node.children.filter(c => c.tag === "a").map(a => inline(a, mapHref).trim()).filter(Boolean);
    if (links.length) out.push(links.join(" · "));
    return;
  }

  if (hasClass(node, "head") || hasClass(node, "meta")) {
    const h3 = node.children.find(c => c.tag === "h3");
    if (h3) {
      const tag = node.children.find(c => c.tag === "span" && hasClass(c, "tag"));
      const suffix = tag ? ` (${inline(tag, mapHref).trim()})` : "";
      out.push(`### ${inline(h3, mapHref, true).trim()}${suffix}`);
      return;
    }
    const t = inline(node, mapHref).trim();
    if (t) out.push(t);
    return;
  }

  for (const c of node.children) blocks(c, mapHref, out);
}

export function htmlToMd(html, mapHref = h => h) {
  const tree = parse(html);
  const out = [];
  blocks(tree, mapHref, out);
  return out.filter(Boolean).join("\n\n").replace(/\n{3,}/g, "\n\n").trim();
}
