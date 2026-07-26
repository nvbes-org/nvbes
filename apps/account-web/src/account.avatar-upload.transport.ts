export function uploadProfileAvatarToPresignedUrl(url: string, file: File): Promise<void> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open('PUT', url, true);
    request.setRequestHeader('Content-Type', file.type);

    request.onload = () => {
      if (request.status >= 200 && request.status < 300) {
        resolve();
        return;
      }

      reject(new Error(`Profile photo upload failed with status ${request.status}`));
    };
    request.onerror = () => reject(new Error('Profile photo upload failed: network error'));
    request.onabort = () => reject(new Error('Profile photo upload cancelled'));
    request.send(file);
  });
}
