/**
 * @file constants/hardwareSpecs.ts
 * @brief Immutable Physical Hardware Specifications & Functional SSOT Builder barrel.
 */

export * from './hardwareSpecs/physicalConstants';
export {
  buildCanonicalHardwareSsotSpec,
  CANONICAL_HARDWARE_SSOT_SPEC,
  type SupportedLocale,
} from './hardwareSpecs/ssotBuilder';
