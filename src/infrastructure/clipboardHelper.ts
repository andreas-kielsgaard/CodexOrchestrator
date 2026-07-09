export interface ClipboardWriter {
  writeText(text: string): Promise<void>;
}

export class BrowserClipboardHelper implements ClipboardWriter {
  async writeText(text: string): Promise<void> {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }

    copyWithTemporaryTextArea(text);
  }
}

export const browserClipboardHelper = new BrowserClipboardHelper();

function copyWithTemporaryTextArea(text: string): void {
  const textArea = document.createElement('textarea');
  textArea.value = text;
  textArea.setAttribute('readonly', 'true');
  textArea.style.position = 'fixed';
  textArea.style.inset = '0 auto auto 0';
  textArea.style.opacity = '0';

  document.body.append(textArea);
  textArea.select();

  try {
    document.execCommand('copy');
  } finally {
    textArea.remove();
  }
}
