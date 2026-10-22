import { Button } from "@/components/ui/button";

export function ConfirmDialog({
  label,
  detail,
  confirmLabel,
  onConfirm,
}: {
  label: string;
  detail?: string;
  confirmLabel: string;
  onConfirm: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div role="dialog" aria-label={label} className="w-full max-w-md rounded-lg border border-border bg-card p-4 shadow-lg">
        <h2 className="font-sans text-title font-semibold">{label}</h2>
        {detail ? <p className="mt-1 text-sm text-muted-foreground">{detail}</p> : null}
        <div className="mt-4 flex justify-end">
          <Button type="button" size="sm" onClick={onConfirm}>{confirmLabel}</Button>
        </div>
      </div>
    </div>
  );
}
