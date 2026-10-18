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
    <div role="dialog" aria-label={label} className="mt-3 rounded-md border border-border p-3">
      {detail ? <p>{detail}</p> : null}
      <Button type="button" size="sm" onClick={onConfirm}>{confirmLabel}</Button>
    </div>
  );
}
