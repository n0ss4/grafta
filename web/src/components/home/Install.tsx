import type { ComponentType } from "react";
import { cn } from "~/lib/cn";
import { ClaudeCodeMark, ClaudeMark, CodeXml, CursorMark, Terminal, WindsurfMark } from "../editorial/icons";
import { Section } from "../editorial/Section";
import { Wordmark } from "../editorial/Wordmark";
import { Configurator } from "../install/Configurator";

type Glyph = ComponentType<{ size?: number; className?: string }>;
/** Where the client has a mark grafta may show (simple-icons, CC0), it is shown; otherwise its name and a glyph. */
const marks: Record<string, { Icon: Glyph; colour?: string }> = {
  "claude-code": { Icon: ClaudeCodeMark, colour: "text-[#D97757]" },
  codex: { Icon: Terminal, colour: "text-bud" },
  "claude-desktop": { Icon: ClaudeMark, colour: "text-[#D97757]" },
  cursor: { Icon: CursorMark },
  vscode: { Icon: CodeXml, colour: "text-bud" },
  windsurf: { Icon: WindsurfMark },
};

const glyphs = Object.fromEntries(
  Object.entries(marks).map(([id, { Icon, colour }]) => [
    id,
    <Icon key={id} size={16} className={cn(colour ?? "text-current")} />,
  ]),
);

/** 07: install; the wordmark again at the foot. */
export function Install({
  highlighted,
  client,
  onClient,
}: {
  highlighted: Record<string, string>;
  client: string;
  onClient: (id: string) => void;
}) {
  return (
    <Section id="install" index="07" label="Install" tone="ink" className="overflow-hidden pb-0 lg:pb-0">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <h2
          id="install-title"
          className="col-span-4 text-[36px] leading-[1.1] font-normal tracking-[-0.03em] sm:text-[48px] lg:col-span-5 lg:col-start-2"
        >
          Install it.
        </h2>
        <div className="col-span-4 mt-10 min-w-0 lg:col-span-6 lg:col-start-2">
          <Configurator highlighted={highlighted} value={client} onValueChange={onClient} glyphs={glyphs} />
        </div>
        <p className="micro col-span-4 mt-8 text-muted lg:col-start-2">
          macOS · Linux · Windows · MIT · not affiliated with Figma
        </p>
      </div>
      <Wordmark className="mt-24 -mb-[9vw] -ml-[1vw] text-paper/[0.06]" />
    </Section>
  );
}
