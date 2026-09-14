export interface ComposerQuickPage {
  readonly title: string;
  readonly items: readonly ComposerQuickAction[];
  readonly emptyMessage?: string;
  readonly limitations?: readonly string[];
}

interface ComposerQuickActionDetails {
  readonly id: string;
  readonly label: string;
  readonly description: string;
  readonly keywords?: readonly string[];
  readonly disabledReason?: string;
  readonly selected?: boolean;
  readonly acceptsSlash?: boolean;
}

/** A choice opens a page or performs a local draft action. */
export type ComposerQuickAction = ComposerQuickActionDetails &
  (
    | {
        readonly children: readonly ComposerQuickAction[];
        readonly loadChildren?: never;
        readonly run?: never;
      }
    | {
        readonly children?: never;
        readonly loadChildren: () => Promise<ComposerQuickPage>;
        readonly run?: never;
      }
    | {
        readonly children?: never;
        readonly loadChildren?: never;
        readonly run: () => { readonly replacement: string; readonly notice: string };
      }
  );
