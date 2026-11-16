/**
 * @file store/slices/toastSlice.ts
 * @brief Immutable Redux slice tracking active notification toasts alongside Sonner display.
 */

import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import type { ToastItem } from '@/types/workbench';

export interface ToastSliceState {
  readonly items: readonly ToastItem[];
}

const initialState: ToastSliceState = {
  items: [],
};

const toastSlice = createSlice({
  name: 'toast',
  initialState,
  reducers: {
    enqueueToast(state, action: PayloadAction<ToastItem>) {
      return { ...state, items: [...state.items, action.payload] };
    },
    dismissToast(state, action: PayloadAction<string>) {
      return {
        ...state,
        items: state.items.filter(item => item.id !== action.payload),
      };
    },
    clearToasts(state) {
      return { ...state, items: [] };
    },
  },
});

export const { enqueueToast, dismissToast, clearToasts } = toastSlice.actions;
export const toastReducer = toastSlice.reducer;
