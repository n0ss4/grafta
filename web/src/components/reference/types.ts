// web/src/generated/tools.json.

export interface Argument {
  name: string;
  type: string;
  required: boolean;
  description: string;
}

export interface Tool {
  name: string;
  description: string;
  /** What it does on a file open in Figma, from the plugin's contract: same, differs or cannot. */
  live: string;
  /** That, as the reference says it: a label, then the reason. */
  there: string;
  arguments: Argument[];
}

export interface Family {
  title: string;
  blurb: string;
  tools: Tool[];
}
