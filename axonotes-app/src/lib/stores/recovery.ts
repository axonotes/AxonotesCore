import {writable} from "svelte/store";

/**
 * Recovery Flow State
 * Manages state during the password recovery wizard
 */

export interface RecoveryState {
  // Step 1: Enter mnemonic
  oldMnemonic: string;

  // Step 2: Set new password
  newPassword: string;

  // Step 3: New mnemonic (generated after password reset)
  newMnemonic: string;

  // Flow state
  currentStep: 1 | 2 | 3;
}

const initialState: RecoveryState = {
  oldMnemonic: "",
  newPassword: "",
  newMnemonic: "",
  currentStep: 1,
};

function createRecoveryStore() {
  const {subscribe, set, update} = writable<RecoveryState>(initialState);

  return {
    subscribe,

    setOldMnemonic(mnemonic: string) {
      update((state) => ({...state, oldMnemonic: mnemonic}));
    },

    setNewPassword(password: string) {
      update((state) => ({...state, newPassword: password}));
    },

    setNewMnemonic(mnemonic: string) {
      update((state) => ({...state, newMnemonic: mnemonic}));
    },

    setStep(step: 1 | 2 | 3) {
      update((state) => ({...state, currentStep: step}));
    },

    reset() {
      set(initialState);
    },
  };
}

export const recoveryStore = createRecoveryStore();
