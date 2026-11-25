import type { ReactNode } from "react";
import { Band, Wrap } from "./Section";

// Three things grafta does, each with a line drawing: layers stacked (it
// reads the file), one layer lifted and marked (it changes it), many shapes
// rising from one plane (what it is used for is open). Drawn in the theme's
// colours: the lines in the text's colour, faint; the change in sap.

const plane = "M0 -60 L104 0 L0 60 L-104 0 Z";

const figures: { label: string; title: string; text: string; art: ReactNode }[] = [
  {
    label: "FIG 0.1",
    title: "Reads your design",
    text: "Colours, text, sizes, components and variables: the file's real values, not a screenshot.",
    art: (
      <g fill="none" strokeWidth="1">
        {[150, 132, 114, 96].map((y) => (
          <path key={y} d={plane} transform={`translate(150 ${y})`} className="fill-bg stroke-text/25" />
        ))}
        <g transform="translate(150 78)" className="stroke-text/50">
          <path d={plane} className="fill-bg" />
          <path d="M0 -36 L62 0 L0 36 L-62 0 Z" />
          <path d="M-30 -2 L18 26 M-14 -11 L34 17" strokeDasharray="3 4" />
        </g>
      </g>
    ),
  },
  {
    label: "FIG 0.2",
    title: "Changes it for you",
    text: "Ask for a change and it is made in Figma. You see each change, and can undo it.",
    art: (
      <g fill="none" strokeWidth="1">
        <g transform="translate(150 150)" className="stroke-text/25">
          <path d={plane} />
          <path d="M-40 -6 L10 23 L40 6 L-10 -23 Z" strokeDasharray="3 4" />
        </g>
        <path d="M150 108 L150 72" strokeDasharray="2 4" className="stroke-accent-text/60" />
        <path
          d="M-40 -6 L10 23 L40 6 L-10 -23 Z"
          transform="translate(150 62)"
          className="fill-accent/10 stroke-accent-text"
        />
        <circle cx="150" cy="62" r="3" className="fill-accent-text" />
      </g>
    ),
  },
  {
    label: "FIG 0.3",
    title: "For whatever you want",
    text: "Code, another version of the design, a document, a review. What you do next is up to you.",
    art: (
      <g strokeWidth="1" className="fill-bg stroke-text/25">
        <path d="M150 210 L254 150 L150 90 L46 150 Z" fill="none" />
        <path d="M84 156 L84 96 L120 75 L120 135 Z" />
        <path d="M112 172 L112 92 L148 71 L148 151 Z" />
        <path d="M140 188 L140 78 L176 57 L176 167 Z" />
        <path d="M168 172 L168 62 L204 41 L204 151 Z" className="stroke-text/50" />
        <path d="M196 156 L196 76 L232 55 L232 135 Z" />
      </g>
    ),
  },
];

/** What grafta does, in three figures. */
export function Figures() {
  return (
    <Band id="what" label="What it does" padding="py-0">
      <Wrap>
        <div className="grid md:grid-cols-3">
          {figures.map((figure, i) => (
            <article
              key={figure.label}
              className={`grid gap-3.5 border-line py-10 md:px-8 md:pb-12 ${i > 0 ? "border-t md:border-t-0 md:border-l" : "md:pl-0"}`}
            >
              <span className="font-mono text-[11px] tracking-[0.04em] text-faint">{figure.label}</span>
              <svg
                viewBox="0 0 300 230"
                aria-hidden="true"
                className="mx-auto my-3 h-[230px] w-full max-w-[300px] overflow-visible"
              >
                {figure.art}
              </svg>
              <h3 className="text-[15px] font-[510] tracking-normal">{figure.title}</h3>
              <p className="max-w-[34ch] text-[15px] text-muted">{figure.text}</p>
            </article>
          ))}
        </div>
      </Wrap>
    </Band>
  );
}
