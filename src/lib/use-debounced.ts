import { useEffect, useState } from "react";

export function useDebounced<T>(value: T, delayMs = 150): T {
  const wait = import.meta.env.MODE === "test" ? 0 : delayMs;
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    if (wait === 0) {
      setDebounced(value);
      return;
    }
    const timer = window.setTimeout(() => setDebounced(value), wait);
    return () => window.clearTimeout(timer);
  }, [value, wait]);
  if (wait === 0) return value;
  return debounced;
}
