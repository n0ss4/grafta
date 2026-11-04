import type { ComponentType } from "react";
import { cn } from "~/lib/cn";
import { ClaudeCodeMark, ClaudeMark, CodeXml, CursorMark, Terminal, WindsurfMark } from "../editorial/icons";

type Glyph = ComponentType<{ size?: number; className?: string }>;

const marks: Record<string, { Icon: Glyph; colour: string }> = {
  "claude-code": { Icon: ClaudeCodeMark, colour: "text-[#D97757]" },
  codex: { Icon: Terminal, colour: "text-bud" },
  "claude-desktop": { Icon: ClaudeMark, colour: "text-[#D97757]" },
  cursor: { Icon: CursorMark, colour: "text-paper" },
  vscode: { Icon: CodeXml, colour: "text-[#23A8F2]" },
  windsurf: { Icon: WindsurfMark, colour: "text-[#46D6B0]" },
  unix: { Icon: Terminal, colour: "text-bud" },
  brew: { Icon: Terminal, colour: "text-bud" },
  windows: { Icon: Terminal, colour: "text-[#23A8F2]" },
  rust: { Icon: Terminal, colour: "text-[#E5603A]" },
};

export function ClientMark({ id, size = 18, className }: { id: string; size?: number; className?: string }) {
  const mark = marks[id] ?? marks.codex;
  if (!mark) return null;
  const { Icon, colour } = mark;
  return <Icon size={size} className={cn("flex-none", colour, className)} />;
}
