// Colours and paints, as grafta writes and reads them: `#RRGGBB` (or
// `#RRGGBBAA` given), stops and transforms in the plugin API's own shape.
import { Names, num } from "./nodes";

const byte = (channel: number): string =>
  Math.round(Math.min(Math.max(channel, 0), 1) * 255)
    .toString(16)
    .toUpperCase()
    .padStart(2, "0");

export function hex(color: RGB | RGBA): string {
  return `#${byte(color.r)}${byte(color.g)}${byte(color.b)}`;
}

/** `#RRGGBB` or `#RRGGBBAA` as channels, or words naming what was given. */
export function parseHex(at: string, text: unknown): RGBA {
  const digits = typeof text === "string" && text.startsWith("#") ? text.slice(1) : "";
  if (!/^([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(digits)) throw new Error(`${at}: expected #RRGGBB or #RRGGBBAA, got ${JSON.stringify(text)}`);
  const at2 = (i: number) => parseInt(digits.slice(i, i + 2), 16) / 255;
  return { r: at2(0), g: at2(2), b: at2(4), a: digits.length === 8 ? at2(6) : 1 };
}

/** Paints as read_node shows them. */
export async function paintsOut(paints: readonly Paint[], names: Names): Promise<unknown[]> {
  const out: unknown[] = [];
  for (const paint of paints) {
    const p: Record<string, unknown> = { type: paint.type };
    if (paint.type === "SOLID") p.color = hex(paint.color);
    const opacity = paint.opacity ?? 1;
    if (opacity < 1) p.opacity = num(opacity);
    if (paint.visible === false) p.visible = false;
    if (paint.blendMode && paint.blendMode !== "NORMAL") p.blendMode = paint.blendMode;
    if (paint.type.startsWith("GRADIENT_")) {
      const gradient = paint as GradientPaint;
      p.gradientStops = gradient.gradientStops.map((stop) => {
        const s: Record<string, unknown> = { position: num(stop.position), color: hex(stop.color) };
        if (stop.color.a < 1) s.opacity = num(stop.color.a);
        return s;
      });
      p.gradientTransform = gradient.gradientTransform.map((row) => row.map(num));
    }
    if (paint.type === "IMAGE") {
      if (paint.imageHash) p.imageHash = paint.imageHash;
      p.scaleMode = paint.scaleMode;
    }
    if (paint.type === "SOLID" && paint.boundVariables?.color) p.variable = await names.label(paint.boundVariables.color);
    out.push(p);
  }
  return out;
}

/** The transform of a linear gradient at `degrees`: 180 top to bottom, 90 left to right. */
export function angleTransform(degrees: number): Transform {
  const radians = (degrees * Math.PI) / 180;
  const clean = (v: number) => (Math.abs(v) < 1e-9 ? 0 : v);
  const [dx, dy] = [Math.sin(radians), -Math.cos(radians)];
  return [
    [clean(dx), clean(dy), clean(0.5 - 0.5 * dx - 0.5 * dy)],
    [clean(-dy), clean(dx), clean(0.5 + 0.5 * dy - 0.5 * dx)],
  ];
}

const PAINT_KEYS: Record<string, readonly string[]> = {
  SOLID: ["type", "color", "opacity", "variable", "visible", "blendMode"],
  GRADIENT_LINEAR: ["type", "gradientStops", "angle", "gradientTransform", "opacity", "visible", "blendMode"],
  GRADIENT_RADIAL: ["type", "gradientStops", "gradientTransform", "opacity", "visible", "blendMode"],
  GRADIENT_ANGULAR: ["type", "gradientStops", "gradientTransform", "opacity", "visible", "blendMode"],
  GRADIENT_DIAMOND: ["type", "gradientStops", "gradientTransform", "opacity", "visible", "blendMode"],
  IMAGE: ["type", "imageHash", "image", "scaleMode", "opacity", "visible", "blendMode"],
};

/** Paints as set_properties takes them, made into the plugin API's. */
export async function paintsIn(at: string, given: unknown, variables: (name: string) => Promise<Variable>): Promise<Paint[]> {
  if (!Array.isArray(given)) throw new Error(`${at}: expected an array of paints`);
  const out: Paint[] = [];
  for (const [index, item] of given.entries()) {
    const where = `${at}[${index}]`;
    const p = item as Record<string, unknown>;
    const type = String(p.type);
    const keys = PAINT_KEYS[type];
    if (!keys) throw new Error(`${where}.type: \`${type}\` is not one of ${Object.keys(PAINT_KEYS).join(", ")}`);
    const extra = Object.keys(p).find((key) => !keys.includes(key));
    if (extra) throw new Error(`${where}: \`${extra}\` is not accepted; a ${type} paint takes ${keys.join(", ")}`);
    const common = {
      opacity: typeof p.opacity === "number" ? p.opacity : undefined,
      visible: typeof p.visible === "boolean" ? p.visible : undefined,
      blendMode: typeof p.blendMode === "string" ? (p.blendMode as BlendMode) : undefined,
    };
    if (type === "SOLID") {
      const color = parseHex(`${where}.color`, p.color);
      let paint: SolidPaint = { type: "SOLID", color: { r: color.r, g: color.g, b: color.b }, ...defined(common) };
      if (color.a < 1 && common.opacity === undefined) paint = { ...paint, opacity: color.a };
      if (typeof p.variable === "string") paint = figma.variables.setBoundVariableForPaint(paint, "color", await variables(p.variable));
      out.push(paint);
    } else if (type.startsWith("GRADIENT_")) {
      if (!Array.isArray(p.gradientStops)) throw new Error(`${where}.gradientStops: expected an array of { position, color, opacity? }`);
      const gradientStops: ColorStop[] = p.gradientStops.map((stop: Record<string, unknown>, i: number) => {
        const color = parseHex(`${where}.gradientStops[${i}].color`, stop.color);
        const alpha = typeof stop.opacity === "number" ? stop.opacity : color.a;
        return { position: Number(stop.position), color: { r: color.r, g: color.g, b: color.b, a: alpha } };
      });
      if (p.angle !== undefined && p.gradientTransform !== undefined) throw new Error(`${where}: give angle or gradientTransform, not both`);
      if (p.angle !== undefined && type !== "GRADIENT_LINEAR") throw new Error(`${where}.angle: only a linear gradient takes an angle`);
      const gradientTransform: Transform = Array.isArray(p.gradientTransform)
        ? (p.gradientTransform as Transform)
        : type === "GRADIENT_LINEAR"
          ? angleTransform(typeof p.angle === "number" ? p.angle : 180)
          : [
              [1, 0, 0],
              [0, 1, 0],
            ];
      out.push({ type: type as GradientPaint["type"], gradientStops, gradientTransform, ...defined(common) });
    } else {
      if (typeof p.image === "string")
        throw new Error(`${where}.image: an image by path is read from disk by grafta, which the open file cannot; give its imageHash`);
      if (typeof p.imageHash !== "string") throw new Error(`${where}.imageHash: an IMAGE paint needs the hash of an image the file holds`);
      const scaleMode = (typeof p.scaleMode === "string" ? p.scaleMode : "FILL") as ImagePaint["scaleMode"];
      out.push({ type: "IMAGE", imageHash: p.imageHash, scaleMode, ...defined(common) });
    }
  }
  return out;
}

function defined<T extends Record<string, unknown>>(value: T): Partial<T> {
  return Object.fromEntries(Object.entries(value).filter(([, v]) => v !== undefined)) as Partial<T>;
}
