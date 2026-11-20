// One list drives everything: <head> tags, JSON-LD, sitemap, llms.txt, the .md twins and the social cards.
// Bodies live in src/pages/<slug>.html; nav and footer in src/partials/.
export const GH = "https://github.com/bobcat88/footnote";
export const SUPPORT = "johan.velocity.lab@pm.me";
export const pages = [
  { slug: "index", path: "/", name: "Footnote One", mark: "n",
    title: "Footnote One · Local memory your coding agents can cite",
    description: "Footnote One gives your coding agent a local memory of your documents, your past work and your code. Install it from the command line; every answer says where it came from.",
    og: { title: "Footnote One: local memory your coding agents can cite", description: "Library for your documents, Memory for what your agent learns, Code for your codebase. Every answer shows its source.", alt: "Footnote One: your agent stops starting from zero.", card: ["Footnote One", "Your agent stops <em>starting from zero.</em>", "Local memory your coding agents can cite."] },
    line: "Install Footnote One (command line today, Windows installer not released) and the Launcher window." },
  { slug: "library", path: "/library", name: "Footnote Library", mark: "m-lib",
    title: "Footnote Library · Search your docs and notes, cited",
    description: "Search your notes, PDFs and the documentation of the libraries you use. Each result names the file, the line and whether the file changed since. Runs on your machine.",
    og: { title: "Footnote Library: search your docs and notes, cited", description: "Notes, PDFs and library docs, searched locally. Each result names the file, the line and whether it changed.", alt: "Footnote Library: search your notes, PDFs and library docs. Each result names its source.", card: ["Footnote Library", "Search your docs and notes. <em>See the source.</em>", "File, line and freshness on every result."] },
    line: "Search notes, PDFs and library docs; results carry file, line and freshness; measured quality and speed." },
  { slug: "memory", path: "/memory", name: "Footnote Memory", mark: "m-mem",
    title: "Footnote Memory · Your agent's memory, on your machine",
    description: "Your coding agent saves preferences, decisions and fixes in one file on your machine and gets the relevant ones back at session start and on each prompt. See, edit and delete everything.",
    og: { title: "Footnote Memory: your agent's memory, on your machine", description: "Preferences, decisions and fixes, saved by your agent in one local file and recalled when they matter.", alt: "Footnote Memory: what your coding agent learns, kept in one file on your machine.", card: ["Footnote Memory", "What your agent learns <em>stays with you.</em>", "One local file. See it, edit it, delete it."] },
    line: "Agent memory in one local file; what is saved, how to inspect and delete it; recall results." },
  { slug: "code", path: "/code", name: "Footnote Code", mark: "m-code", soon: true,
    title: "Footnote Code (soon) · Who calls what, without guessing",
    description: "Coming soon: ask who calls a function and what it calls, answered from the compiler's own index. It would rather show no caller than a wrong one. Measured precision by language.",
    og: { title: "Footnote Code (soon): who calls what, without guessing", description: "Callers and callees from the compiler's own index. It shows no caller rather than a wrong one.", alt: "Footnote Code, coming soon: callers and callees from the compiler's own index.", card: ["Footnote Code · soon", "Who calls this? <em>Not a guess.</em>", "Answers from the compiler's own index."] },
    line: "Coming soon: callers and callees from the compiler's index; precision and recall by language." },
];
