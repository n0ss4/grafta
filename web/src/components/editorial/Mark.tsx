/**
 * grafta's mark (docs/brand/mark.svg) inline, so it follows the page's theme:
 * two nodes joined and a third grafted on; the edges and nodes in the text
 * colour, the grafted node in the accent (Sap, or Bud in dark and on Ink).
 */
export function Mark({ size = 20 }: { size?: number }) {
  return (
    <svg viewBox="0 0 64 64" width={size} height={size} aria-hidden="true" className="flex-none">
      <path
        d="M14 46 L32 18 L50 46"
        fill="none"
        stroke="currentColor"
        strokeWidth={5.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx={14} cy={46} r={8} fill="currentColor" />
      <circle cx={32} cy={18} r={8} fill="currentColor" />
      <circle cx={50} cy={46} r={8} fill="var(--accent)" />
    </svg>
  );
}
