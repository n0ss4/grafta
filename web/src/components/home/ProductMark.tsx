import type { Product } from "~/content/types";
import { Globe, PenTool, Presentation, StickyNote } from "../editorial/icons";

// Each product's file by its idea, white on its product's colour: a pen for
// Design, a board for Slides, a sticky note for FigJam, a globe for Sites.
const marks: Record<Product["extension"], { Icon: typeof PenTool; tile: string }> = {
  ".fig": { Icon: PenTool, tile: "bg-[#0B84DD]" },
  ".deck": { Icon: Presentation, tile: "bg-[#E8590C]" },
  ".jam": { Icon: StickyNote, tile: "bg-[#8B3FE8]" },
  ".site": { Icon: Globe, tile: "bg-[#4F46D8]" },
};

export function ProductMark({ extension, size = 40 }: { extension: Product["extension"]; size?: number }) {
  const { Icon, tile } = marks[extension];
  return (
    <span
      aria-hidden="true"
      className={`grid place-items-center rounded-[10px] text-white ${tile}`}
      style={{ width: size, height: size }}
    >
      <Icon size={size * 0.55} strokeWidth={1.75} />
    </span>
  );
}
