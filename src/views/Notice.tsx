import { AlertCircle } from 'lucide-react';

export interface ErrorNoticeProps {
  error: string;
}

export function ErrorNotice({ error }: ErrorNoticeProps) {
  return (
    <section className="notice error" role="status">
      <AlertCircle size={18} aria-hidden="true" />
      <span>{error}</span>
    </section>
  );
}
