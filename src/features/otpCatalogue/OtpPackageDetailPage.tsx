import { useMemo, useState, type ReactNode } from 'react';
import type { OtpPackageDto } from '../../application/otp';
import { OtpPackageDetailPanel } from './OtpPackageDetailPanel';
import { OtpPackageElementList } from './OtpPackageElementList';
import { packageElementSections, type OtpCatalogueElement } from './otpCataloguePresentation';

export function OtpPackageDetailPage({
  pkg,
  onBack,
  packageConfiguration,
}: {
  readonly pkg: OtpPackageDto;
  readonly onBack: () => void;
  readonly packageConfiguration?: ReactNode;
}) {
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const sections = useMemo(() => packageElementSections(pkg), [pkg]);
  const selected =
    sections
      .flatMap((section) => section.elements.map((item) => item.element))
      .find((element) => element.key === selectedKey) ?? null;
  const toggleSelection = (element: OtpCatalogueElement) => {
    setSelectedKey((current) => (current === element.key ? null : element.key));
  };
  return (
    <section className="otp-catalogue__detail-page" aria-label={`${pkg.name} package details`}>
      <button type="button" className="otp-catalogue__back" onClick={onBack}>
        ← OTP configuration
      </button>
      <div className="otp-catalogue__master-detail">
        <OtpPackageElementList
          packageName={pkg.name}
          sections={sections}
          selectedKey={selectedKey}
          onSelect={toggleSelection}
        />
        <OtpPackageDetailPanel
          pkg={pkg}
          selected={selected}
          packageConfiguration={packageConfiguration}
        />
      </div>
    </section>
  );
}
