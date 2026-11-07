import type { Layer } from "~/content/talk";

/** The glyph Figma's layers panel gives each kind of layer, simplified. */
const GLYPH: Record<string, string> = { FRAME: "#", TEXT: "T", VECTOR: "◇" };

/**
 * The file's layers, as `read_node` returned them. `marked` shows what the
 * request changed in grafta's colour.
 */
export function Layers({ layer, marked, depth = 0 }: { layer: Layer; marked: boolean; depth?: number }) {
  const changed = marked && layer.changed;
  return (
    <li>
      <span
        className="flex items-center gap-1.5 truncate rounded px-1.5 py-[3px] text-[12px] leading-4 data-[changed=true]:bg-soft data-[changed=true]:text-accent-text"
        style={{ paddingLeft: `${6 + depth * 12}px` }}
        data-changed={changed}
      >
        <span aria-hidden="true" className="w-3 shrink-0 text-center text-[10px] text-muted">
          {GLYPH[layer.type] ?? "·"}
        </span>
        <span className="truncate">{layer.name}</span>
      </span>
      {layer.children.length > 0 && (
        <ul>
          {layer.children.map((child) => (
            <Layers key={child.id} layer={child} marked={marked} depth={depth + 1} />
          ))}
        </ul>
      )}
    </li>
  );
}
