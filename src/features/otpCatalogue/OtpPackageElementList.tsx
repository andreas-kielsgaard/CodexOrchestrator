import {
  elementLabel,
  elementTypeLabel,
  type OtpCatalogueElement,
  type OtpCatalogueElementSection,
} from './otpCataloguePresentation';

export function OtpPackageElementList({
  packageName,
  sections,
  selectedKey,
  onSelect,
}: {
  readonly packageName: string;
  readonly sections: readonly OtpCatalogueElementSection[];
  readonly selectedKey: string | null;
  readonly onSelect: (element: OtpCatalogueElement) => void;
}) {
  return (
    <nav className="otp-catalogue__element-list" aria-label={`${packageName} offered elements`}>
      {sections.map((section) => (
        <section className="otp-catalogue__element-section" key={section.key}>
          <h3>{section.label}</h3>
          <ul>
            {section.elements.map(({ element, depth }) => {
              const selected = element.key === selectedKey;
              return (
                <li key={element.key}>
                  <button
                    type="button"
                    data-depth={depth}
                    aria-pressed={selected}
                    title={selected ? 'Select again to show package details' : 'Show details'}
                    onClick={() => onSelect(element)}
                  >
                    <span>{elementLabel(element)}</span>
                    <small aria-hidden="true">{elementTypeLabel(element)}</small>
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </nav>
  );
}
