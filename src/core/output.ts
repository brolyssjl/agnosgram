import { serialize, type Format } from "./serialize.js";

export class UserError extends Error {}

export function info(msg: string): void {
  process.stdout.write(msg + "\n");
}

export function warn(msg: string): void {
  process.stderr.write(msg + "\n");
}

export function printJson(value: unknown): void {
  process.stdout.write(serialize(value, "json") + "\n");
}

/** Print a structured value in a resolved non-human format (`json` or `toon`). */
export function printStructured(value: unknown, format: Format): void {
  process.stdout.write(serialize(value, format) + "\n");
}
