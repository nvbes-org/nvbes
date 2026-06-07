export function ResetPasswordErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

export function ResetPasswordSuccessMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-sm text-emerald-600">
      {message}
    </div>
  );
}
