export function LoginPageFooter({ searchStr }: { searchStr: string }) {
  return (
    <p className="text-center text-sm text-muted-foreground">
      Pas de compte ?{' '}
      <a href={`/register${searchStr}`} className="font-medium text-primary hover:underline">
        S&apos;inscrire
      </a>
    </p>
  );
}
