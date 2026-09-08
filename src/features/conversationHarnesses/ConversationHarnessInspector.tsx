/**
 * Compatibility entry point for the session-owned Harness Management surface.
 *
 * The editor itself lives in HarnessEditor so other product surfaces reuse the
 * reviewed composition instead of rebuilding its fields as a generic form.
 */
export { HarnessEditor as ConversationHarnessManagement } from './HarnessEditor';
export type { HarnessEditorProps as ConversationHarnessManagementProps } from './HarnessEditor';
