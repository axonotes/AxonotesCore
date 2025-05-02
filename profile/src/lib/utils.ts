import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import {PUBLIC_ENVIRONMENT} from "$env/static/public";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function inDev() {
	if (PUBLIC_ENVIRONMENT !== "dev" && PUBLIC_ENVIRONMENT !== "prod") {
		throw new Error("PUBLIC_ENVIRONMENT must be either 'dev' or 'prod'");
	}

	return PUBLIC_ENVIRONMENT === "dev";
}

export function devLog(...args: unknown[]) {
	if (inDev() && console.log) {
		console.log(...args);
	}
}