import { Badge } from "@/components/ui/badge";
import { sourceLabel } from "@/lib/source";

export function SourceBadge({ source }: { source: string }) {
  return (
    <Badge variant="outline" className="shrink-0 whitespace-nowrap font-sans font-normal">
      {sourceLabel(source)}
    </Badge>
  );
}
