import type { OtpPackageDto } from '../../application/otp';
import { packageInventory } from './otpCataloguePresentation';

export function OtpPackageDirectory({ packages, onOpen }: { readonly packages: readonly OtpPackageDto[]; readonly onOpen: (id: string) => void }) {
  return (
    <div className="otp-catalogue__directory">
      {packages.map((pkg) => {
        const inventory = packageInventory(pkg);
        const counts = [
          inventory.workflowSources ? `${inventory.workflowSources} trigger/event source${inventory.workflowSources === 1 ? '' : 's'}` : null,
          inventory.workflowDestinations ? `${inventory.workflowDestinations} destination${inventory.workflowDestinations === 1 ? '' : 's'}` : null,
          inventory.mcpServices ? `${inventory.mcpServices} MCP service${inventory.mcpServices === 1 ? '' : 's'}` : null,
          inventory.capabilityGroups ? `${inventory.capabilityGroups} capability group${inventory.capabilityGroups === 1 ? '' : 's'}` : null,
          inventory.endpoints ? `${inventory.endpoints} endpoint${inventory.endpoints === 1 ? '' : 's'}` : null,
        ].filter(Boolean);
        return (
          <article key={pkg.id} className="otp-catalogue__package-card">
            <header><div><h3>{pkg.name}</h3><p>{pkg.summary}</p></div><span>Imported</span></header>
            <p className="otp-catalogue__package-id">{pkg.id}</p>
            <ul>{counts.map((count) => <li key={count}>{count}</li>)}</ul>
            <button type="button" onClick={() => onOpen(pkg.id)}>View details</button>
          </article>
        );
      })}
    </div>
  );
}
