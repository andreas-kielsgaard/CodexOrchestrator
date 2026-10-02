import type { NewAgentImageAttachmentDto } from '../../application/agentSessions';

export const isImageAttachment = (attachment: { mediaType: string }) =>
  ['image/png', 'image/jpeg', 'image/gif', 'image/webp'].includes(attachment.mediaType);
export const isLongPaste = (text: string) => text.length > 4000 || text.split(/\r?\n/).length > 50;

export async function readBrowserAttachment(file: File): Promise<NewAgentImageAttachmentDto> {
  if (file.size > 10 * 1024 * 1024) throw new Error(`${file.name} is larger than 10 MB.`);
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 0x8000)
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  const id = [...digest].map((value) => value.toString(16).padStart(2, '0')).join('');
  return {
    id,
    name: file.name,
    mediaType: file.type || 'application/octet-stream',
    dataBase64: btoa(binary),
  };
}
export const textAttachment = (text: string) =>
  readBrowserAttachment(new File([text], 'Pasted text.txt', { type: 'text/plain' }));

export async function readComposerClipboard(): Promise<{ text: string | null; paths: string[] }> {
  if ('__TAURI_INTERNALS__' in window) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('read_agent_session_clipboard');
  }
  return { text: await navigator.clipboard.readText(), paths: [] };
}
