import specs from "./sessions.json";

/**
 * A state of a session's design, the same in every client.
 * `needs`: tools the calls that reach it must include; "a|b" accepts either.
 */
export interface Beat {
  id: string;
  title: string;
  says: string;
  needs: string[];
}

/** What a session leaves the person with, in their words. */
export interface Outcome {
  summary: string;
  /** Whether it writes a file; the file's name is the recording's, from its last save_as. */
  writes: boolean;
}

export interface SessionSpec {
  id: string;
  title: string;
  /** The person's words, sent as they are in each client it was recorded in. */
  prompt: string;
  beats: Beat[];
  outcome: Outcome;
}

/**
 * The five requests the hero plays, recorded with these prompts in Claude
 * Desktop and Claude Code. The data is sessions.json, read by
 * docs/showcase/transcripts_site.py too.
 */
export const sessions: SessionSpec[] = specs;
