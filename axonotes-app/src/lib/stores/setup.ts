import {writable} from "svelte/store";

/**
 * Setup Flow State
 * Manages state during the new user setup wizard
 */

export interface SetupState {
  // Step 1: Master password (stored temporarily until keys are created)
  masterPassword: string;

  // Step 2: Mnemonic (generated after creating user)
  mnemonic: string;

  // Step 3: Local security choice
  localSecurityMode: "pin" | "password" | "none";
  localSecurityValue: string;

  // Flow state
  currentStep: 1 | 2 | 3;
  isCreatingUser: boolean;
}

const initialState: SetupState = {
  masterPassword: "",
  mnemonic: "",
  localSecurityMode: "none",
  localSecurityValue: "",
  currentStep: 1,
  isCreatingUser: false,
};

function createSetupStore() {
  const {subscribe, set, update} = writable<SetupState>(initialState);

  return {
    subscribe,

    setMasterPassword(password: string) {
      update((state) => ({...state, masterPassword: password}));
    },

    setMnemonic(mnemonic: string) {
      update((state) => ({...state, mnemonic}));
    },

    setLocalSecurity(mode: "pin" | "password" | "none", value: string = "") {
      update((state) => ({
        ...state,
        localSecurityMode: mode,
        localSecurityValue: value,
      }));
    },

    setStep(step: 1 | 2 | 3) {
      update((state) => ({...state, currentStep: step}));
    },

    setCreatingUser(isCreating: boolean) {
      update((state) => ({...state, isCreatingUser: isCreating}));
    },

    reset() {
      set(initialState);
    },
  };
}

export const setupStore = createSetupStore();
