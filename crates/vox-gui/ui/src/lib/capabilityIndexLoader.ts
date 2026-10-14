/**
 * The ONLY runtime importer of the generated capability index (D-06): the 784 KB artifact
 * must stay out of the startup bundle, so it loads through one memoized dynamic import on the
 * first palette open. Every other file uses type-only imports of it.
 */
export type CapabilityIndexModule = typeof import('../generated/capabilityIndex.generated');

let loaded: CapabilityIndexModule | null = null;
let pending: Promise<CapabilityIndexModule> | null = null;

export function loadCapabilityIndex(): Promise<CapabilityIndexModule> {
  pending ??= import('../generated/capabilityIndex.generated')
    .then((m) => (loaded = m))
    .catch((err) => {
      pending = null; // let a later open retry
      throw err;
    });
  return pending;
}

export function getLoadedCapabilityIndex(): CapabilityIndexModule | null {
  return loaded;
}
