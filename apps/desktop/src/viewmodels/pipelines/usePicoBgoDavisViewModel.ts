import { useBgoBenchViewModel } from './useBgoBenchViewModel';
import { useDavisProbeViewModel } from './useDavisProbeViewModel';

export { useBgoBenchViewModel, useDavisProbeViewModel };

export function usePicoBgoDavisViewModel(
  initialPort?: string | null,
  onOpenHddPipeline?: () => void,
) {
  const bgo = useBgoBenchViewModel(initialPort);
  const davis = useDavisProbeViewModel({
    initialPort: initialPort ?? undefined,
    onOpenHddPipeline,
  });
  return { bgo, davis };
}
