/**
 * @file store/slices/nvramCaptureSlice.ts
 * @brief Immutable Redux slice holding the latest live in-circuit P7501 NVRAM capture.
 */

import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import type { LiveNvramCaptureEvent } from '@/models';

export interface NvramCaptureSliceState {
  readonly latestCapture: LiveNvramCaptureEvent | null;
}

const initialState: NvramCaptureSliceState = {
  latestCapture: null,
};

const nvramCaptureSlice = createSlice({
  name: 'nvramCapture',
  initialState,
  reducers: {
    recordLiveCapture(state, action: PayloadAction<LiveNvramCaptureEvent>) {
      if (state.latestCapture === action.payload) {
        return state;
      }
      return { ...state, latestCapture: action.payload };
    },
    clearLiveCapture(state) {
      return { ...state, latestCapture: null };
    },
  },
});

export const { recordLiveCapture, clearLiveCapture } = nvramCaptureSlice.actions;
export const nvramCaptureReducer = nvramCaptureSlice.reducer;
