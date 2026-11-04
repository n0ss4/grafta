// A grafta call as the site shows it.

export interface Picture {
  src: string;
  width: number;
  height: number;
  alt: string;
}

export interface Step {
  tool: string;
  args: Record<string, unknown>;
  /** A bounded excerpt of grafta's JSON result. */
  result: string;
  /** What the excerpt cut, as grafta or the excerpt says it. */
  omitted?: string | undefined;
  picture?: Picture | undefined;
}
