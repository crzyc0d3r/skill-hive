import type * as T from "@/lib/api/bindings";

/** Every library item, sorted by name. */
export const ALL_ITEMS_QUERY: T.LibraryQuery = {
  text: null,
  kinds: [],
  tags: [],
  sourceKinds: [],
  favoritesOnly: false,
  deployedTo: null,
  sort: "name",
  limit: null,
};

/** Sum of what an agent loads into its prompt for a set of skills: name + description per entry. */
export function estimateDescriptionChars(items: { name: string; description: string }[]): number {
  return items.reduce((n, i) => n + i.name.length + i.description.length + 4, 0);
}

export function sameSet(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false;
  const s = new Set(a);
  return b.every((x) => s.has(x));
}
