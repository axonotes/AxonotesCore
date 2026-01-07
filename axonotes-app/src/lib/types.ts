export interface Profile {
    id: string;
    name: string;
    email: string;
}

export type UnlockMode = 'none' | 'pin' | 'pass';