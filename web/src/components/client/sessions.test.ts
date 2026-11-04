import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { sessions } from "~/content/sessions";
import index from "~/generated/session-index.json";
import type { Recording } from "./types";

// The recordings the site ships, as docs/showcase/transcripts_site.py wrote them.
const dir = join(process.cwd(), "public", "sessions");
const files = readdirSync(dir).filter((f) => f.endsWith(".json"));
const recordings = files.map(
  (f) => [f, JSON.parse(readFileSync(join(dir, f), "utf8")) as Recording] as const,
);

describe("the recorded sessions", () => {
  it("are one file per session and client the index lists", () => {
    const listed = Object.entries(index as Record<string, string[]>).flatMap(([id, clients]) =>
      clients.map((c) => `${id}.${c}.json`),
    );
    expect(files.sort()).toEqual(listed.sort());
  });

  it("follow the contract: who, when, what changed, and turns that start with the prompt and end with a reply", () => {
    for (const [file, r] of recordings) {
      expect(["claude-desktop", "claude-code"], file).toContain(r.client);
      expect(r.clientVersion, file).not.toBe("");
      expect(r.model, file).not.toBe("");
      expect(r.recordedOn, file).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(Object.keys(r.edits).sort(), file).toEqual(["cut", "otherTools", "paths", "pictures"]);
      expect(r.turns[0]?.kind, file).toBe("user");
      expect(r.turns.at(-1)?.kind, file).toBe("reply");
    }
  });

  it("start with the prompt in sessions.json, and reach every beat of the session in order of the rows", () => {
    for (const [file, r] of recordings) {
      const spec = sessions.find((s) => s.id === r.id.replace(/-without$/, ""));
      expect(spec, file).toBeDefined();
      if (!spec) continue;
      expect(r.turns[0]?.kind === "user" && r.turns[0].text, file).toBe(spec.prompt);
      if (r.id.endsWith("-without")) continue;
      const beats = new Set(r.turns.flatMap((t) => (t.kind === "tools" && !t.other ? [t.beat] : [])));
      expect([...beats].sort(), file).toEqual(spec.beats.map((b) => b.id).sort());
    }
  });

  it("name the file a session wrote only when it writes one, and the without-grafta one calls no grafta tool", () => {
    for (const [file, r] of recordings) {
      const spec = sessions.find((s) => s.id === r.id);
      if (spec) expect(Boolean(r.file), file).toBe(spec.outcome.writes);
      if (r.id.endsWith("-without")) {
        expect(
          r.turns.some((t) => t.kind === "tools" && !t.other),
          file,
        ).toBe(false);
      }
    }
  });

  it("carry no path or name from the machine they were recorded on", () => {
    for (const [file, r] of recordings) {
      expect(JSON.stringify(r), file).not.toMatch(/\/Users\/|\/home\/|\/private\/|C:\\\\Users/);
    }
  });

  it("show a picture only where the site has it", () => {
    const pictures = new Set(readdirSync(join(process.cwd(), "public", "showcase", "sessions")));
    for (const [file, r] of recordings) {
      for (const t of r.turns) {
        if (t.kind === "tools")
          for (const c of t.calls)
            if (c.picture) expect(pictures, file).toContain(c.picture.src.split("/").pop());
        if (t.kind === "reply")
          for (const m of t.text.matchAll(/!\[[^\]]*\]\(showcase\/sessions\/([^)]+)\)/g))
            expect(pictures, file).toContain(m[1]);
      }
    }
  });
});
