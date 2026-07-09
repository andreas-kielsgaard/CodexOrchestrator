import { FileText, Upload } from 'lucide-react';
import type { HTMLAttributes } from 'react';
import { StatusPill } from './StatusPill';
import type { OrchestrationFileItem } from './types';

export interface FileListProps extends HTMLAttributes<HTMLUListElement> {
  emptyLabel?: string;
  files: OrchestrationFileItem[];
}

const kindLabels: Record<OrchestrationFileItem['kind'], string> = {
  backend_evidence: 'Backend evidence',
  draft: 'Draft reference',
  runtime_evidence: 'Runtime evidence',
  uploaded: 'Uploaded file',
};

export function FileList({
  className,
  emptyLabel = 'No uploaded files yet.',
  files,
  ...props
}: FileListProps) {
  const classes = ['ui-orchestration-file-list', className].filter(Boolean).join(' ');

  if (files.length === 0) {
    return <p className="ui-orchestration-empty">{emptyLabel}</p>;
  }

  return (
    <ul {...props} className={classes}>
      {files.map((file) => (
        <li key={file.id}>
          <span className="ui-orchestration-file-list__icon">
            {file.kind === 'uploaded' ? (
              <Upload aria-hidden="true" size={16} />
            ) : (
              <FileText aria-hidden="true" size={16} />
            )}
          </span>
          <div>
            <strong>{file.name}</strong>
            <small>{file.detailLabel ?? kindLabels[file.kind]}</small>
            {file.evidenceLabel ? <small>{file.evidenceLabel}</small> : null}
          </div>
          {file.state ? <StatusPill state={file.state} /> : null}
        </li>
      ))}
    </ul>
  );
}
