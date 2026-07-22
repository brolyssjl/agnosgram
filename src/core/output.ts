export class UserError extends Error {}

export function info(msg: string): void {
  process.stdout.write(msg + "\n");
}

export function warn(msg: string): void {
  process.stderr.write(msg + "\n");
}

export function printJson(value: unknown): void {
  process.stdout.write(JSON.stringify(value, null, 2) + "\n");
}
