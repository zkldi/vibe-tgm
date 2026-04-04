#!/usr/bin/env python3
"""
Extract every user prompt from Cursor Agent transcripts for a git checkout.

Cursor stores transcripts as JSONL under:
  ~/.cursor/projects/<workspace-path-with-slashes-replaced-by-dashes>/agent-transcripts/

Each line is JSON; user turns have role \"user\" and text in message.content[].text.
The actual prompt is usually inside <user_query>...</user_query> (optionally after
<attached_files>...</attached_files>).

Usage:
  .scripts/dump_cursor_prompts.py
  .scripts/dump_cursor_prompts.py -o prompts.txt
  .scripts/dump_cursor_prompts.py --transcripts-dir ~/.cursor/projects/.../agent-transcripts
  .scripts/dump_cursor_prompts.py --raw   # full user message blobs, not just <user_query>

Sorting: JSONL has no per-message timestamps. Prompts are ordered by (1) file birth time
(conversation file creation, best proxy for “when the chat started” on macOS) and
(2) line number within that file (order of messages in the conversation).
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path

USER_QUERY_RE = re.compile(r"<user_query>\s*(.*?)\s*</user_query>", re.DOTALL)


def cursor_transcripts_dir(repo_root: Path) -> Path:
    """Mirror Cursor's project folder naming (macOS/Linux style paths)."""
    resolved = repo_root.resolve()
    p = str(resolved)
    if p.startswith("/"):
        p = p[1:]
    slug = p.replace("/", "-")
    return Path.home() / ".cursor" / "projects" / slug / "agent-transcripts"


def iter_jsonl(path: Path):
    with path.open(encoding="utf-8", errors="replace") as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                yield line_no, json.loads(line)
            except json.JSONDecodeError as e:
                print(f"Warning: skip {path}:{line_no}: {e}", file=sys.stderr)


def file_birthtime(path: Path) -> float:
    """Creation time when available (macOS st_birthtime); else mtime."""
    st = path.stat()
    return float(getattr(st, "st_birthtime", st.st_mtime))


def format_when(ts: float) -> str:
    return dt.datetime.fromtimestamp(ts).astimezone().strftime("%Y-%m-%d %H:%M:%S %Z")


def collect_user_text_parts(record: dict) -> str:
    if record.get("role") != "user":
        return ""
    msg = record.get("message") or {}
    parts = msg.get("content")
    if not isinstance(parts, list):
        return ""
    chunks: list[str] = []
    for block in parts:
        if not isinstance(block, dict):
            continue
        if block.get("type") != "text":
            continue
        t = block.get("text")
        if isinstance(t, str):
            chunks.append(t)
    return "".join(chunks)


def extract_prompt_text(raw: str, raw_mode: bool) -> str:
    if raw_mode:
        return raw.strip()
    m = USER_QUERY_RE.search(raw)
    if m:
        return m.group(1).strip()
    return raw.strip()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "-r",
        "--repo",
        type=Path,
        default=None,
        help="Repository root (default: git root from cwd)",
    )
    ap.add_argument(
        "-t",
        "--transcripts-dir",
        type=Path,
        default=None,
        help="Override path to agent-transcripts directory",
    )
    ap.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Write to this file (default: stdout)",
    )
    ap.add_argument(
        "--raw",
        action="store_true",
        help="Dump full user message text (includes <attached_files>, etc.)",
    )
    ap.add_argument(
        "--group-by-transcript",
        action="store_true",
        help="Group prompts by transcript file (order: file birth time, then line order)",
    )
    ap.add_argument(
        "--newest-first",
        action="store_true",
        help="Reverse chronological order (only affects time-sorted output)",
    )
    args = ap.parse_args()

    if args.transcripts_dir is not None:
        tdir = args.transcripts_dir.expanduser()
    else:
        repo = args.repo
        if repo is None:
            try:
                import subprocess

                root = subprocess.run(
                    ["git", "rev-parse", "--show-toplevel"],
                    capture_output=True,
                    text=True,
                    check=True,
                ).stdout.strip()
                repo = Path(root)
            except (subprocess.CalledProcessError, FileNotFoundError):
                print(
                    "Not inside a git repo; pass --repo /path/to/checkout or --transcripts-dir",
                    file=sys.stderr,
                )
                return 1
        else:
            repo = args.repo.expanduser()
        tdir = cursor_transcripts_dir(repo)

    if not tdir.is_dir():
        print(f"Transcripts directory not found: {tdir}", file=sys.stderr)
        print(
            "Open this project in Cursor and use Agent at least once, "
            "or pass --transcripts-dir explicitly.",
            file=sys.stderr,
        )
        return 1

    jsonl_files = sorted(tdir.glob("**/*.jsonl"))
    if not jsonl_files:
        print(f"No .jsonl files under {tdir}", file=sys.stderr)
        return 1

    # Collect (birth, line_no, path, transcript_id, prompt_index_in_file, text)
    entries: list[tuple[float, int, Path, str, int, str]] = []
    for jf in jsonl_files:
        tid = jf.parent.name
        birth = file_birthtime(jf)
        n_in_file = 0
        for line_no, rec in iter_jsonl(jf):
            raw = collect_user_text_parts(rec)
            if not raw:
                continue
            text = extract_prompt_text(raw, args.raw)
            if not text:
                continue
            n_in_file += 1
            entries.append((birth, line_no, jf, tid, n_in_file, text))

    out_lines: list[str] = []
    out_lines.append(f"Cursor user prompts — source: {tdir}")
    out_lines.append("")
    total_prompts = len(entries)

    if args.group_by_transcript:
        # Sort files by birth time, then emit each file's prompts in line order
        by_file: dict[Path, list[tuple[float, int, Path, str, int, str]]] = {}
        for e in entries:
            by_file.setdefault(e[2], []).append(e)
        files_order = sorted(by_file.keys(), key=lambda p: (file_birthtime(p), str(p)))
        if args.newest_first:
            files_order.reverse()
        for jf in files_order:
            chunk = sorted(by_file[jf], key=lambda e: e[1])
            tid = jf.parent.name
            out_lines.append("=" * 80)
            out_lines.append(f"Transcript: {tid}")
            out_lines.append(f"File: {jf}")
            out_lines.append(f"Conversation file created (approx.): {format_when(file_birthtime(jf))}")
            out_lines.append("=" * 80)
            out_lines.append("")
            for _b, _ln, _p, _tid, n, text in chunk:
                out_lines.append(f"--- Prompt {n} ---")
                out_lines.append(text)
                out_lines.append("")
    else:
        entries.sort(key=lambda e: (e[0], e[1], str(e[2])))
        if args.newest_first:
            entries.reverse()
        # out_lines.append(
        #     "Order: oldest first by (file creation time, then line in transcript). "
        #     "See script docstring for caveats."
        # )
        out_lines.append("")
        prev_time = None
        for birth, line_no, jf, tid, n_in_file, text in entries:
            if prev_time is not None and birth != prev_time:
                out_lines.append("=" * 80)
                out_lines.append(f"When (conversation file created): {format_when(birth)}")
                out_lines.append("=" * 80)
            else:
                out_lines.append("---")
            out_lines.append("")
            out_lines.append(text)
            out_lines.append("")

    out_lines.append("")
    out_lines.append(f"Total user prompts: {total_prompts}")

    text_out = "\n".join(out_lines).rstrip() + "\n"

    if args.output:
        args.output.expanduser().write_text(text_out, encoding="utf-8")
        print(f"Wrote {total_prompts} prompts to {args.output}", file=sys.stderr)
    else:
        sys.stdout.write(text_out)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
