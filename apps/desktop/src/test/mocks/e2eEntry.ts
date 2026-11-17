import type { PanadigaTestIpcBridge } from '@/services/ipc';
import { handleMockCommand, subscribeMockEventStream } from './mockDispatcher';
import { installTestIpcBridge } from './testHostBridges';

const bridge: PanadigaTestIpcBridge = {
  invoke: <T>(cmd: string, args?: Record<string, unknown>) => handleMockCommand<T>(cmd, args),
  listen: <T>(eventName: string, handler: (payload: T) => void) =>
    subscribeMockEventStream<T>(eventName, handler),
};

installTestIpcBridge(bridge);
