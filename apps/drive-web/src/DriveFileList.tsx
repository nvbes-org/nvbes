import { UploadErrorBoundary } from '@nvbes/web-runtime';
import { FileCode2, FileText, Folder, Share2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export type DriveFileItem = {
  name: string;
  detail: string;
  type: 'folder' | 'document' | 'code';
  shareUrl?: string;
};

async function shareFile(file: DriveFileItem) {
  const shareData: ShareData = {
    title: file.name,
    text: `Partage depuis nvbes Drive : ${file.name}`,
    url: file.shareUrl ?? window.location.href,
  };

  if (typeof navigator.share === 'function') {
    try {
      await navigator.share(shareData);
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      await fallbackCopy(shareData);
    }
  } else {
    await fallbackCopy(shareData);
  }
}

async function fallbackCopy(shareData: ShareData) {
  const url = shareData.url ?? window.location.href;
  try {
    await navigator.clipboard.writeText(url);
  } catch {
    // Clipboard not available either — silently fail
  }
}

export function DriveFileList({ files, query }: { files: DriveFileItem[]; query: string }) {
  return (
    <UploadErrorBoundary>
      <Card className="border-border/60 shadow-sm">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
          <div>
            <CardTitle className="text-base">Fichiers recents</CardTitle>
            <CardDescription>
              Vue de travail temporaire en attendant la vraie source Drive.
            </CardDescription>
          </div>
          <Button variant="outline" className="h-8 px-3">
            Afficher tout
          </Button>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-2">
          {files.length === 0 && (
            <div className="col-span-full rounded-2xl border border-dashed border-border/70 bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
              Aucun fichier ne correspond a "{query}".
            </div>
          )}
          {files.map((file) => (
            <article
              key={file.name}
              className="group/file rounded-2xl border border-border/60 bg-background p-4 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md"
            >
              <div className="flex items-start gap-3">
                <div className="flex size-10 items-center justify-center rounded-xl bg-muted">
                  {file.type === 'folder' ? (
                    <Folder className="size-5 text-primary" />
                  ) : file.type === 'code' ? (
                    <FileCode2 className="size-5 text-primary" />
                  ) : (
                    <FileText className="size-5 text-primary" />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <h3 className="truncate font-medium">{file.name}</h3>
                  <p className="text-sm text-muted-foreground">{file.detail}</p>
                </div>
                <button
                  type="button"
                  onClick={() => void shareFile(file)}
                  className="flex size-8 shrink-0 items-center justify-center rounded-lg text-muted-foreground opacity-0 transition hover:bg-muted hover:text-foreground group-hover/file:opacity-100 focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-primary/20"
                  aria-label={`Partager ${file.name}`}
                >
                  <Share2 className="size-4" />
                </button>
              </div>
            </article>
          ))}
        </CardContent>
      </Card>
    </UploadErrorBoundary>
  );
}
