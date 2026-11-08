import type { ComponentType } from "react";
import { cn } from "~/lib/cn";
import {
  ClaudeCodeMark,
  ClaudeMark,
  CodeXml,
  CursorMark,
  MessageCircle,
  Terminal,
  WindsurfMark,
} from "../editorial/icons";

type Glyph = ComponentType<{ size?: number; className?: string }>;

// Each way in's mark, in colours that hold on the page's light and dark themes.
const marks: Record<string, { Icon: Glyph; colour: string }> = {
  "claude-desktop": { Icon: ClaudeMark, colour: "text-[#D97757]" },
  "claude-code": { Icon: ClaudeCodeMark, colour: "text-[#D97757]" },
  chatgpt: { Icon: MessageCircle, colour: "text-text" },
  codex: { Icon: Terminal, colour: "text-text" },
  cursor: { Icon: CursorMark, colour: "text-text" },
  vscode: { Icon: CodeXml, colour: "text-[#1F8AD2]" },
  windsurf: { Icon: WindsurfMark, colour: "text-[#14A887]" },
  unix: { Icon: Terminal, colour: "text-accent-text" },
  brew: { Icon: Terminal, colour: "text-accent-text" },
  windows: { Icon: Terminal, colour: "text-accent-text" },
  rust: { Icon: Terminal, colour: "text-accent-text" },
};

export function ClientMark({ id, size = 18, className }: { id: string; size?: number; className?: string }) {
  const mark = marks[id] ?? marks.codex;
  if (!mark) return null;
  const { Icon, colour } = mark;
  return <Icon size={size} className={cn("flex-none", colour, className)} />;
}
