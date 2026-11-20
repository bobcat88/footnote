import glob, shutil, sqlite3, sys, os
W = sys.argv[1]
for n in "click httpx rich hono nest zustand".split():
    for k in ("p", "d"):
        os.makedirs(f"{W}/{k}", exist_ok=True)
        shutil.rmtree(f"{W}/{k}/{n}-comp", ignore_errors=True)
        shutil.copytree(f"{W}/s-{n}-t", f"{W}/{k}/{n}-comp")
    shutil.copy(f"{W}/parser/{n}.tsv", f"{W}/p/{n}.tsv")
    db = glob.glob(f"{W}/s-{n}-d/code/*.db")[0]
    c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    with open(f"{W}/d/{n}.tsv", "w", encoding="utf-8", newline="\n") as f:
        for a, l, nm, b, o in c.execute("""SELECT rf.path, r.line, s.name, sf.path, r.origin FROM refs r JOIN files rf ON rf.id=r.file_id
            JOIN symbols s ON s.id=r.symbol_id JOIN files sf ON sf.id=s.file_id"""):
            f.write(f"{a.replace(chr(92),'/')}\t{l}\t{nm}\t{b.replace(chr(92),'/')}\tdeep_{o}\n")
    print(n, sum(1 for _ in open(f"{W}/p/{n}.tsv", encoding="utf-8")), sum(1 for _ in open(f"{W}/d/{n}.tsv", encoding="utf-8")))
