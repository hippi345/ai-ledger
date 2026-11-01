/** Saves a URL (data: or blob:) as a file via a temporary download link. */
export function saveUrl(fileName: string, url: string): void {
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.rel = "noopener";
  document.body.appendChild(a);
  a.click();
  a.remove();
}

/** Saves text as a file. The blob URL is revoked after the click. */
export function saveText(
  fileName: string,
  text: string,
  mime = "text/csv;charset=utf-8",
): void {
  const url = URL.createObjectURL(new Blob([text], { type: mime }));
  try {
    saveUrl(fileName, url);
  } finally {
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }
}
