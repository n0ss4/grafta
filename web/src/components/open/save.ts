// Handing files to the person: one as itself, several in a zip made here.

/** Give the browser `blob` to save as `name`. */
export function save(name: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 1000);
}

/** Files in one zip, stored as they are: PNG is compressed already, and JSON is small. */
export async function zip(files: { name: string; bytes: Uint8Array }[]): Promise<Blob> {
  const { zipSync } = await import("fflate");
  const entries: import("fflate").Zippable = {};
  for (const file of files) entries[file.name] = [file.bytes, { level: 0 }];
  return new Blob([zipSync(entries)], { type: "application/zip" });
}
