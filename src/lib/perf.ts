export const WARM_START_BUDGET_MS = 1500;

export function warmStartWithinBudget(elapsedMs: number): boolean {
  return elapsedMs >= 0 && elapsedMs < WARM_START_BUDGET_MS;
}

export function windowRows<T>(rows: T[], cap = 100): T[] {
  return rows.length > cap ? rows.slice(0, cap) : rows;
}
