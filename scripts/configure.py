#!/usr/bin/env python3
"""Set public app identity. No secrets, deployments, network calls, or account changes."""
import argparse,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def configure(root,name,slug):
    if not name.strip() or len(name)>80 or any(not c.isprintable() for c in name):
        raise ValueError("Name must be 1–80 printable characters")
    if not re.fullmatch(r"[a-z0-9](?:[a-z0-9-]{1,46})[a-z0-9]",slug):
        raise ValueError("Slug must be 3–48 lowercase letters, digits, or internal hyphens")
    (root/"app.json").write_text(json.dumps({"name":name.strip(),"slug":slug},ensure_ascii=False,indent=2)+"\n")
    path=root/"Dioxus.toml"
    text=path.read_text()
    text=re.sub(r"(?m)^title = .*", "title = "+json.dumps(name.strip(),ensure_ascii=False), text)
    path.write_text(text)
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name",required=True);parser.add_argument("--slug",required=True)
    args=parser.parse_args()
    try:configure(ROOT,args.name,args.slug)
    except ValueError as error:parser.error(str(error))
    print("Public app identity configured. Next: just up; see docs/new-app-setup.md.")
if __name__=="__main__":main()
