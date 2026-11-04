// web/src/generated/tools.json, as specs/067-react-website/contracts/tools.schema.json has it.

export interface Argument {
  name: string;
  type: string;
  required: boolean;
  description: string;
}

export interface Tool {
  name: string;
  description: string;
  arguments: Argument[];
}

export interface Family {
  title: string;
  blurb: string;
  tools: Tool[];
}
