export interface RemovalPlan {
  paths: string[];
  token: string;
}

export function removalPlan(paths: string[]): RemovalPlan {
  return { paths: paths.filter((path) => path.trim().length > 0), token: paths.join("|") };
}

export function applyRemoval(plan: RemovalPlan, token: string | null): boolean {
  return token === plan.token;
}
