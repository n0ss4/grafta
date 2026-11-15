// The ports grafta's bridge may listen on, in order, from contract.json: the
// panel's only need of it, read alone so the rest is not bundled with it.
import { ports } from "./contract.json";

export const PORTS: readonly number[] = ports;
