import { useCalibrationViewModel } from './useCalibrationViewModel';
import { useTbcSdHardenViewModel } from './useTbcSdHardenViewModel';

export { useCalibrationViewModel, useTbcSdHardenViewModel };

export function useCalibrationTbcViewModel() {
  const calibration = useCalibrationViewModel();
  const tbcSd = useTbcSdHardenViewModel();
  return { calibration, tbcSd };
}
