export type CloseChoice = "ask" | "minimize" | "quit";

export function effectiveClose(choice: CloseChoice, trayEnabled: boolean): CloseChoice {
  if (!trayEnabled && choice === "minimize") return "ask";
  return choice;
}
