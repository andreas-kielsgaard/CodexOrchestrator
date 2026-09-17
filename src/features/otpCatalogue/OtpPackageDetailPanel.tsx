import type { ReactNode } from 'react';
import type { OtpPackageDto } from '../../application/otp';
import { OtpElementDetails } from '../otp/otpElements';
import type { OtpCatalogueElement } from './otpCataloguePresentation';

export function OtpPackageDetailPanel({
  pkg,
  selected,
  packageConfiguration,
}: {
  readonly pkg: OtpPackageDto;
  readonly selected: OtpCatalogueElement | null;
  readonly packageConfiguration?: ReactNode;
}) {
  if (selected)
    return (
      <section className="otp-catalogue__detail-panel">
        <SelectedElementDetails element={selected} />
      </section>
    );
  return (
    <section className="otp-catalogue__detail-panel otp-catalogue__overview">
      <p className="otp-catalogue__eyebrow">Imported Orchestration Tool Package</p>
      <h2>{pkg.name}</h2>
      <p>{pkg.description}</p>
      <p className="otp-catalogue__package-id">
        Package ID: {pkg.id} · Contract v{pkg.contractVersion}
      </p>
      {packageConfiguration ? (
        <section className="otp-catalogue__configuration">{packageConfiguration}</section>
      ) : null}
    </section>
  );
}

function SelectedElementDetails({ element }: { readonly element: OtpCatalogueElement }) {
  if ('tool' in element) return <OtpElementDetails tool={element.tool} />;
  if ('endpoint' in element)
    return <OtpElementDetails endpoint={element.endpoint} server={element.server} />;
  if ('group' in element)
    return <OtpElementDetails group={element.group} server={element.server} />;
  return <OtpElementDetails server={element.server} />;
}
